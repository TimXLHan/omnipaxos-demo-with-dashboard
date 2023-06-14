use crate::{messages::IOMessage, coordinator::KeyValue};
use crate::messages::coordinator::CDMessage;
use crate::ui::UI;
use tokio::sync::mpsc::{Receiver, Sender};

pub struct Controller {
    ui: UI,
    io_receiver: Receiver<IOMessage>,
    cd_sender: Sender<CDMessage>,
}

impl Controller {
    pub fn new(ui: UI, io_receiver: Receiver<IOMessage>, cd_sender: Sender<CDMessage>) -> Self {
        Self {
            ui,
            io_receiver,
            cd_sender,
        }
    }

    pub(crate) async fn handle(&mut self, m: IOMessage) {
        match m {
            IOMessage::CDMessage(cd_m) => {
                self.cd_sender.send(cd_m).await.unwrap();
            }
            IOMessage::UIMessage(ui_m) => {
                println!("{:?}", ui_m);
                //self.ui.handle(ui_m);
            }
        }
    }
    
    pub async fn run(&mut self) {
        self.cd_sender.send(CDMessage::Initialize).await.expect("Couldn't start Coordinator");

        std::thread::sleep(std::time::Duration::from_secs(1));
        let kv = KeyValue { key: "a".to_string(), value: "test".to_string() };
        self.cd_sender.send(CDMessage::KVCommand(crate::messages::coordinator::KVCommand::Put(kv))).await.expect("Couldn't send to Coordinator");
        
        std::thread::sleep(std::time::Duration::from_secs(1));
        self.cd_sender.send(CDMessage::KVCommand(crate::messages::coordinator::KVCommand::Get("a".to_string()))).await.unwrap();

        while let Some(m) = self.io_receiver.recv().await {
            self.handle(m).await;
        }
    }
}
