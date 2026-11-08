/// An error from kui
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("event loop: {0}")]
    EventLoop(#[from] winit::error::EventLoopError),

    #[error("window: {0}")]
    Window(#[from] winit::error::OsError),

    #[error("gpu: {0}")]
    Gpu(#[from] vello::Error),

    #[error("layout: {0}")]
    Layout(#[from] taffy::TaffyError),
}
