use crate::coordinator::Coordinator;
use crate::messages::IOMessage;
use crate::ui::UI;
use tokio::sync::mpsc::Receiver;

pub struct Controller {
    ui: UI,
    cd: Coordinator,
    io_receiver: Receiver<IOMessage>,
}

impl Controller {
    pub fn new(ui: UI, cd: Coordinator, io_receiver: Receiver<IOMessage>) -> Self {
        Self {
            ui,
            cd,
            io_receiver,
        }
    }

    pub(crate) fn handle(&mut self, m: IOMessage) {
        match m {
            IOMessage::CDMessage(cd_m) => {
                self.cd.handle(cd_m);
            }
            IOMessage::UIMessage(ui_m) => {
                self.ui.handle(ui_m);
            }
        }
    }

    pub async fn run(&mut self) {
        while let Some(m) = self.io_receiver.recv().await {
            self.handle(m);
        }
    }
}
