use std::{
    collections::HashMap,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use crate::{
    jsonrpc::{self, ID, Message},
    transport::{TransportReader, TransportWriter},
};

use lsp_types::{notification::Notification, request::Request};
use serde::Serialize;
use tokio::{
    io,
    sync::{mpsc, oneshot},
    task::JoinHandle,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Object Parse Error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Transport error: {0}")]
    Transport(#[from] tokio::io::Error),

    #[error("Server error {}: {}", .0.code, .0.message)]
    Response(jsonrpc::Error),

    #[error("Connection closed")]
    Closed,
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
type RequestHandler<S> = Box<
    dyn Fn(&mut S, serde_json::Value) -> BoxFuture<Result<serde_json::Value, jsonrpc::Error>>
        + Send,
>;
type NotificationHandler<S> = Box<dyn Fn(&mut S, serde_json::Value) + Send>;

/// Requests waiting on a response, or None once the connection closes
type Pending = Mutex<Option<HashMap<u64, oneshot::Sender<jsonrpc::Response>>>>;

/// A message from the server waiting on a flush
enum Incoming {
    Request(jsonrpc::Request),
    Notification(jsonrpc::Notification),
}

/// A builder that collects handlers before a Client starts
pub struct ClientBuilder<S> {
    requests: HashMap<&'static str, RequestHandler<S>>,
    notifications: HashMap<&'static str, NotificationHandler<S>>,
}

impl<S> Default for ClientBuilder<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> ClientBuilder<S> {
    /// Create a new ClientBuilder with no handlers
    pub fn new() -> Self {
        Self {
            requests: HashMap::new(),
            notifications: HashMap::new(),
        }
    }

    /// Set the handler for requests from the server of type R
    pub fn on_request<R, F, Fut>(mut self, handler: F) -> Self
    where
        R: Request,
        F: Fn(&mut S, R::Params) -> Fut + Send + 'static,
        Fut: Future<Output = Result<R::Result, jsonrpc::Error>> + Send + 'static,
    {
        let erased: RequestHandler<S> = Box::new(move |state, params| {
            let params = match serde_json::from_value::<R::Params>(params) {
                Ok(params) => params,
                Err(e) => {
                    let error = jsonrpc::Error::new(jsonrpc::Error::INVALID_PARAMS, e.to_string());
                    return Box::pin(std::future::ready(Err(error)));
                }
            };

            let future = handler(state, params);
            Box::pin(async move {
                serde_json::to_value(future.await?)
                    .map_err(|e| jsonrpc::Error::new(jsonrpc::Error::INTERNAL_ERROR, e.to_string()))
            })
        });

        self.requests.insert(R::METHOD, erased);
        self
    }

    /// Set the handler for notifications from the server of type N
    pub fn on_notification<N, F>(mut self, handler: F) -> Self
    where
        N: Notification,
        F: Fn(&mut S, N::Params) + Send + 'static,
    {
        let erased: NotificationHandler<S> = Box::new(move |state, params| {
            if let Ok(params) = serde_json::from_value::<N::Params>(params) {
                handler(state, params);
            }
        });

        self.notifications.insert(N::METHOD, erased);
        self
    }

    /// Start a Client on the given transport, along with the Dispatcher for its events
    pub fn start<R, W>(
        self,
        reader: TransportReader<R>,
        writer: TransportWriter<W>,
    ) -> (Client, Dispatcher<S>)
    where
        R: io::AsyncRead + Unpin + Send + 'static,
        W: io::AsyncWrite + Unpin + Send + 'static,
    {
        let (outgoing, outgoing_rx) = mpsc::unbounded_channel();
        let (incoming_tx, incoming) = mpsc::unbounded_channel();
        let pending = Arc::new(Mutex::new(Some(HashMap::new())));

        tokio::spawn(write_loop(writer, outgoing_rx));
        let read_task = tokio::spawn(read_loop(reader, incoming_tx, pending.clone()));

        let client = Client {
            inner: Arc::new(Inner {
                outgoing: outgoing.clone(),
                pending,
                next_id: AtomicU64::new(0),
                read_task,
            }),
        };

        let dispatcher = Dispatcher {
            requests: self.requests,
            notifications: self.notifications,
            incoming,
            waiting: None,
            outgoing,
        };

        (client, dispatcher)
    }
}

/// Runs handlers for queued server events against a state
pub struct Dispatcher<S> {
    requests: HashMap<&'static str, RequestHandler<S>>,
    notifications: HashMap<&'static str, NotificationHandler<S>>,

    incoming: mpsc::UnboundedReceiver<Incoming>,
    waiting: Option<Incoming>,

    outgoing: mpsc::UnboundedSender<String>,
}

impl<S> Dispatcher<S> {
    /// Wait until at least one event is ready to flush
    pub async fn wait(&mut self) -> Result<()> {
        if self.waiting.is_none() {
            self.waiting = Some(self.incoming.recv().await.ok_or(Error::Closed)?);
        }

        Ok(())
    }

    /// Run the handlers for every queued event and return how many ran (must run inside a tokio runtime)
    pub fn flush(&mut self, state: &mut S) -> usize {
        let mut handled = 0;

        while let Some(incoming) = self
            .waiting
            .take()
            .or_else(|| self.incoming.try_recv().ok())
        {
            match incoming {
                Incoming::Request(request) => self.handle_request(state, request),
                Incoming::Notification(notification) => {
                    if let Some(handler) = self.notifications.get(notification.method.as_str()) {
                        handler(state, notification.params.unwrap_or_default());
                    }
                }
            }

            handled += 1;
        }

        handled
    }

    fn handle_request(&self, state: &mut S, request: jsonrpc::Request) {
        let params = request.params.unwrap_or_default();
        let future = self
            .requests
            .get(request.method.as_str())
            .map(|handler| handler(state, params));

        let outgoing = self.outgoing.clone();
        tokio::spawn(async move {
            let result = match future {
                Some(future) => future.await,
                None => Err(jsonrpc::Error::new(
                    jsonrpc::Error::METHOD_NOT_FOUND,
                    format!("unhandled method {}", request.method),
                )),
            };

            let response = match result {
                Ok(value) => jsonrpc::Response::success(request.id, value),
                Err(error) => jsonrpc::Response::error(request.id, error),
            };

            if let Ok(text) = serde_json::to_string(&response) {
                let _ = outgoing.send(text);
            }
        });
    }
}

struct Inner {
    outgoing: mpsc::UnboundedSender<String>,
    pending: Arc<Pending>,
    next_id: AtomicU64,
    read_task: JoinHandle<Result<()>>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        // The writer stops by itself once every sender is gone
        self.read_task.abort();
    }
}

/// A handle to a running LSP connection
#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

impl Client {
    /// Send a request of type R and return its result
    pub async fn request<R: Request>(&self, params: R::Params) -> Result<R::Result> {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();

        self.inner
            .pending
            .lock()
            .expect("pending lock poisoned")
            .as_mut()
            .ok_or(Error::Closed)?
            .insert(id, tx);

        let request = jsonrpc::Request::new(ID::Num(id), R::METHOD, to_params(params)?);
        if let Err(e) = self.send(&request) {
            if let Some(pending) = self
                .inner
                .pending
                .lock()
                .expect("pending lock poisoned")
                .as_mut()
            {
                pending.remove(&id);
            }
            return Err(e);
        }

        match rx.await.map_err(|_| Error::Closed)? {
            jsonrpc::Response::Success { result, .. } => Ok(serde_json::from_value(result)?),
            jsonrpc::Response::Error { error, .. } => Err(Error::Response(error)),
        }
    }

    /// Send a notification of type N
    pub fn notify<N: Notification>(&self, params: N::Params) -> Result<()> {
        self.send(&jsonrpc::Notification::new(N::METHOD, to_params(params)?))
    }

    fn send(&self, message: &impl Serialize) -> Result<()> {
        self.inner
            .outgoing
            .send(serde_json::to_string(message)?)
            .map_err(|_| Error::Closed)
    }
}

/// Params as JSON, or None for params that serialize to null
fn to_params(params: impl Serialize) -> Result<Option<serde_json::Value>> {
    Ok(match serde_json::to_value(params)? {
        serde_json::Value::Null => None,
        value => Some(value),
    })
}

async fn write_loop<W: io::AsyncWrite + Unpin>(
    mut writer: TransportWriter<W>,
    mut outgoing: mpsc::UnboundedReceiver<String>,
) -> Result<()> {
    while let Some(text) = outgoing.recv().await {
        writer.write(text).await?;
    }

    Ok(())
}

async fn read_loop<R: io::AsyncRead + Unpin>(
    mut reader: TransportReader<R>,
    incoming: mpsc::UnboundedSender<Incoming>,
    pending: Arc<Pending>,
) -> Result<()> {
    let result = loop {
        let text = match reader.recv().await {
            Ok(text) => text,
            Err(e) => break Err(e.into()),
        };

        let Ok(message) = serde_json::from_str::<Message>(&text) else {
            continue;
        };

        match message {
            Message::Response(response) => {
                let ID::Num(id) = response.id() else {
                    continue;
                };

                let waiter = pending
                    .lock()
                    .expect("pending lock poisoned")
                    .as_mut()
                    .and_then(|pending| pending.remove(id));

                if let Some(waiter) = waiter {
                    let _ = waiter.send(response);
                }
            }
            Message::Request(request) => {
                let _ = incoming.send(Incoming::Request(request));
            }
            Message::Notification(notification) => {
                let _ = incoming.send(Incoming::Notification(notification));
            }
        }
    };

    pending.lock().expect("pending lock poisoned").take();

    result
}

#[cfg(test)]
mod test {
    use super::*;

    use lsp_types::{
        PublishDiagnosticsParams, Uri,
        notification::PublishDiagnostics,
        request::{Shutdown, WorkspaceConfiguration},
    };

    type ServerReader = TransportReader<io::ReadHalf<io::DuplexStream>>;
    type ServerWriter = TransportWriter<io::WriteHalf<io::DuplexStream>>;

    fn connect<S>(
        builder: ClientBuilder<S>,
    ) -> (Client, Dispatcher<S>, ServerReader, ServerWriter) {
        let (client_end, server_end) = io::duplex(4096);
        let (client_read, client_write) = io::split(client_end);
        let (server_read, server_write) = io::split(server_end);

        let (client, dispatcher) = builder.start(
            TransportReader::new(client_read),
            TransportWriter::new(client_write),
        );

        (
            client,
            dispatcher,
            TransportReader::new(server_read),
            TransportWriter::new(server_write),
        )
    }

    #[tokio::test]
    async fn test_request_response() {
        let (client, _dispatcher, mut server_reader, mut server_writer) =
            connect(ClientBuilder::<()>::new());

        let request = tokio::spawn(async move { client.request::<Shutdown>(()).await });

        let text = server_reader
            .recv()
            .await
            .expect("Should receive a request");
        let Message::Request(received) = serde_json::from_str(&text).expect("Should parse") else {
            panic!("expected a request, got {text}");
        };
        assert_eq!("shutdown", received.method);
        assert_eq!(None, received.params);

        let response = jsonrpc::Response::success(received.id, serde_json::Value::Null);
        server_writer
            .write(serde_json::to_string(&response).expect("Should serialize"))
            .await
            .expect("Should write a response");

        request
            .await
            .expect("Task should finish")
            .expect("Request should succeed");
    }

    #[tokio::test]
    async fn test_error_response() {
        let (client, _dispatcher, mut server_reader, mut server_writer) =
            connect(ClientBuilder::<()>::new());

        let request = tokio::spawn(async move { client.request::<Shutdown>(()).await });

        let text = server_reader
            .recv()
            .await
            .expect("Should receive a request");
        let Message::Request(received) = serde_json::from_str(&text).expect("Should parse") else {
            panic!("expected a request, got {text}");
        };

        let error = jsonrpc::Error::new(jsonrpc::Error::REQUEST_FAILED, "nope");
        let response = jsonrpc::Response::error(received.id, error.clone());
        server_writer
            .write(serde_json::to_string(&response).expect("Should serialize"))
            .await
            .expect("Should write a response");

        match request.await.expect("Task should finish") {
            Err(Error::Response(received)) => assert_eq!(error, received),
            other => panic!("expected a server error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_server_request() {
        let builder = ClientBuilder::<usize>::new().on_request::<WorkspaceConfiguration, _, _>(
            |calls, params| {
                *calls += 1;
                let calls = *calls;
                async move { Ok(vec![serde_json::json!(calls); params.items.len()]) }
            },
        );
        let (_client, mut dispatcher, mut server_reader, mut server_writer) = connect(builder);
        let mut calls = 0;

        for (id, expected) in [(7, 1), (8, 2)] {
            let request = jsonrpc::Request::new(
                ID::Num(id),
                "workspace/configuration",
                Some(serde_json::json!({ "items": [{}, {}] })),
            );
            server_writer
                .write(serde_json::to_string(&request).expect("Should serialize"))
                .await
                .expect("Should write a request");

            dispatcher.wait().await.expect("Should receive an event");
            assert_eq!(1, dispatcher.flush(&mut calls));
            assert_eq!(expected, calls);

            let text = server_reader
                .recv()
                .await
                .expect("Should receive a response");
            assert_eq!(
                Message::Response(jsonrpc::Response::success(
                    ID::Num(id),
                    serde_json::json!([expected, expected])
                )),
                serde_json::from_str(&text).expect("Should parse")
            );
        }
    }

    #[tokio::test]
    async fn test_unhandled_server_request() {
        let (_client, mut dispatcher, mut server_reader, mut server_writer) =
            connect(ClientBuilder::<()>::new());

        let request = jsonrpc::Request::new(ID::Num(1), "unknown/method", None);
        server_writer
            .write(serde_json::to_string(&request).expect("Should serialize"))
            .await
            .expect("Should write a request");

        dispatcher.wait().await.expect("Should receive an event");
        dispatcher.flush(&mut ());

        let text = server_reader
            .recv()
            .await
            .expect("Should receive a response");
        let Message::Response(jsonrpc::Response::Error { error, id, .. }) =
            serde_json::from_str(&text).expect("Should parse")
        else {
            panic!("expected an error response, got {text}");
        };
        assert_eq!(ID::Num(1), id);
        assert_eq!(jsonrpc::Error::METHOD_NOT_FOUND, error.code);
    }

    #[tokio::test]
    async fn test_notification() {
        let builder = ClientBuilder::<Vec<PublishDiagnosticsParams>>::new()
            .on_notification::<PublishDiagnostics, _>(|received, params| received.push(params));
        let (_client, mut dispatcher, _server_reader, mut server_writer) = connect(builder);

        let uri: Uri = "file:///main.rs".parse().expect("Should parse uri");
        let params = PublishDiagnosticsParams::new(uri, vec![], None);
        let notification = jsonrpc::Notification::new(
            "textDocument/publishDiagnostics",
            Some(serde_json::to_value(&params).expect("Should serialize")),
        );
        server_writer
            .write(serde_json::to_string(&notification).expect("Should serialize"))
            .await
            .expect("Should write a notification");

        let mut received = Vec::new();
        assert_eq!(0, dispatcher.flush(&mut received));

        dispatcher.wait().await.expect("Should receive an event");
        assert_eq!(1, dispatcher.flush(&mut received));
        assert_eq!(vec![params], received);
    }

    #[tokio::test]
    async fn test_closed() {
        let (client, mut dispatcher, server_reader, server_writer) =
            connect(ClientBuilder::<()>::new());

        let request = tokio::spawn({
            let client = client.clone();
            async move { client.request::<Shutdown>(()).await }
        });

        drop(server_writer);
        drop(server_reader);

        assert!(matches!(
            request.await.expect("Task should finish"),
            Err(Error::Closed)
        ));
        assert!(matches!(
            client.request::<Shutdown>(()).await,
            Err(Error::Closed)
        ));
        assert!(matches!(dispatcher.wait().await, Err(Error::Closed)));
    }
}
