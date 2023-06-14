use crate::messages::IOMessage;
use tokio::sync::mpsc::Sender;
use tui_textarea::TextArea;

pub mod render;
mod cli;

/// The ui application, containing the ui state
pub struct UIApp<'a> {
    io_sender: Sender<IOMessage>,
    pub input_area: TextArea<'a>,
}

impl<'a> UIApp<'a> {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self { io_sender, input_area: TextArea::default() }
    }
}
