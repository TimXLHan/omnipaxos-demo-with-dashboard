use tokio::sync::mpsc::Receiver;
use crate::db::DB;
use crate::messages::IOMessage;
use crate::ui::UI;

pub struct Controller {
    ui: UI,
    db: DB,
    io_receiver: Receiver<IOMessage>,
}

impl Controller {
    pub fn new(ui: UI, db: DB, io_receiver: Receiver<IOMessage>) -> Self {
        Self {
            ui,
            db,
            io_receiver,
        }
    }

    pub(crate) fn handle(&mut self, m: IOMessage) {
        match m {
            IOMessage::DBMessage(db_m) => {
                self.db.handle(db_m);
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