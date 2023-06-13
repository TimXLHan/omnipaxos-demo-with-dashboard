use tokio::sync::mpsc::Sender;
use crate::messages::IOMessage;

pub mod render;

/// The ui application, containing the ui state
pub struct UIApp {
    io_sender: Sender<IOMessage>,
}

impl UIApp {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self {
            io_sender
        }
    }
}