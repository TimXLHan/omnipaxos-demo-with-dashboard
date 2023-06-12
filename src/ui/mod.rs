use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use std::sync::Arc;
use crate::messages::{IOMessage, ui::UIMessage};
use crate::ui::ui_app::UIApp;

mod ui_app;

pub struct UI{
    ui_app: Arc<Mutex<UIApp>>,
}

impl UI {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        Self {
            ui_app: Arc::new(Mutex::new(UIApp::new(io_sender))),
        }
    }



    pub(crate) fn handle(&mut self, m: UIMessage) {
        match m{
            UIMessage::Initialize => {
                println!("UIMessage::Initialize");
            }
        }
    }
}