use crate::messages::db::DBMessage;
use tokio::sync::mpsc::Sender;
use crate::messages::IOMessage;

pub struct DB {
    io_sender: Sender<IOMessage>,
}

impl DB {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self {
            io_sender,
        }
    }

    pub(crate) fn handle(&mut self, m: DBMessage) {
        match m{
            DBMessage::Initialize => {
                println!("DBMessage::Initialize");
            }
        }
    }
}