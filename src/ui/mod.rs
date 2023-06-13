use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tui::backend::Backend;
use tui::backend::CrosstermBackend;
use tui::{Frame, Terminal};
use std::sync::Arc;
use std::io::stdout;

use crate::messages::{IOMessage, ui::UIMessage};
use crate::ui::ui_app::render::render;
use crate::ui::ui_app::UIApp;

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
                self.start();
            }
        }
    }

    pub fn start(&self) {
        // Configure Crossterm backend for tui
        let stdout = stdout();
        crossterm::terminal::enable_raw_mode().unwrap();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.clear().unwrap();
        terminal.hide_cursor().unwrap();

        println!("start ui");
        let ui_app = Arc::clone(&self.ui_app);
        tokio::spawn(async move {
            loop {
                let ui_app = ui_app.lock().await;
                // Render
                terminal.draw(|rect| render(rect, &ui_app)).unwrap();
            }
        });
    }

}