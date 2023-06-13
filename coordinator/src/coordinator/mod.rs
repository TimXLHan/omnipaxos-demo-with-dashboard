use crate::messages::coordinator::CDMessage;
use crate::messages::IOMessage;
use tokio::sync::mpsc::Sender;

pub struct Coordinator {
    io_sender: Sender<IOMessage>,
}

impl Coordinator {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self { io_sender }
    }

    pub(crate) fn handle(&mut self, m: CDMessage) {
        match m {
            CDMessage::Initialize => {
                println!("DBMessage::Initialize");
            }
        }
    }
}
