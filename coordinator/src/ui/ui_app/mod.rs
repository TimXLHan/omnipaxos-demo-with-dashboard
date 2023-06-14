use crate::messages::IOMessage;
use tokio::sync::mpsc::Sender;

pub mod render;

/// The ui application, containing the ui state
pub struct UIApp {
    io_sender: Sender<IOMessage>,
}

impl UIApp {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self { io_sender }
    }
}
