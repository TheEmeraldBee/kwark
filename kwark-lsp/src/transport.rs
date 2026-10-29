use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

/// The receiving half of an LSP connection
pub struct TransportReader<R: io::AsyncRead + Unpin> {
    reader: io::BufReader<R>,

    string_buffer: String,
}

impl<R: io::AsyncRead + Unpin> TransportReader<R> {
    /// Create a new TransportReader using the given Reader
    pub fn new(r: R) -> Self {
        Self {
            reader: io::BufReader::new(r),

            string_buffer: String::new(),
        }
    }

    /// Receive the body of the next message
    pub async fn recv(&mut self) -> Result<String, tokio::io::Error> {
        let mut content_length = None;

        loop {
            self.string_buffer.clear();

            if self.reader.read_line(&mut self.string_buffer).await? == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "stream closed while reading headers",
                ));
            }

            let line = self.string_buffer.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }

            if let Some((name, value)) = line.split_once(':')
                && name.trim().eq_ignore_ascii_case("Content-Length")
            {
                let length = value.trim().parse::<usize>().map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("bad Content-Length: {e}"),
                    )
                })?;
                content_length = Some(length);
            }
        }

        let length = content_length.ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length header")
        })?;

        let mut body = vec![0; length];
        self.reader.read_exact(&mut body).await?;

        String::from_utf8(body).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}

/// The sending half of an LSP connection
pub struct TransportWriter<W: io::AsyncWrite + Unpin> {
    writer: io::BufWriter<W>,
}

impl<W: io::AsyncWrite + Unpin> TransportWriter<W> {
    /// Create a new TransportWriter using the given Writer
    pub fn new(w: W) -> Self {
        Self {
            writer: io::BufWriter::new(w),
        }
    }

    /// Send text as a single message
    pub async fn write(&mut self, text: String) -> Result<(), tokio::io::Error> {
        let header = format!("Content-Length: {}\r\n\r\n", text.len());

        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(text.as_bytes()).await?;
        self.writer.flush().await
    }
}
