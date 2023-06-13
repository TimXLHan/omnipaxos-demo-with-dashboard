use crate::messages::{ui::UIMessage, IOMessage};
use crate::ui::ui_app::UIApp;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;

mod ui_app;

pub struct UI {
    ui_app: Arc<Mutex<UIApp>>,
}

impl UI {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self {
            ui_app: Arc::new(Mutex::new(UIApp::new(io_sender))),
        }
    }

    pub(crate) fn handle(&mut self, m: UIMessage) {
        match m {
            UIMessage::Initialize => {
                println!("UIMessage::Initialize");
            }
            _ => unimplemented!(),
        }
    }
}
