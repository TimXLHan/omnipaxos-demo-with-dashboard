use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tui::backend::Backend;
use tui::backend::CrosstermBackend;
use tui::{Frame, Terminal};
use std::io::stdout;
use crossterm::event::{Event, read};
use tui_textarea::{TextArea, Input, Key};

use crate::messages::{IOMessage, ui::UIMessage};
use crate::ui::ui_app::render::render;
use crate::ui::ui_app::UIApp;

mod ui_app;

pub struct UI {
    ui_app: Arc<Mutex<UIApp<'static>>>,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
}

impl UI {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        // Configure Crossterm backend for tui
        let stdout = stdout();
        crossterm::terminal::enable_raw_mode().unwrap();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        Self {
            ui_app: Arc::new(Mutex::new(UIApp::new(io_sender))),
            terminal,
        }
    }

    pub(crate) async fn handle(&mut self, m: UIMessage) {
        match m {
            UIMessage::Initialize => {
                self.start().await;
            }
            _ => unimplemented!(),
        }
    }

    pub async fn start(&mut self) {
        self.terminal.clear().unwrap();
        self.terminal.hide_cursor().unwrap();
        self.update_ui().await;
    }

    async fn update_ui(&mut self) {
        let ui_app = self.ui_app.lock().await;
        self.terminal.draw(|rect| render(rect, &ui_app)).unwrap();
    }
    //
    // async fn user_input_listener(&mut self) {
    //     let mut ui_app = self.ui_app.lock().await;
    //     ui_app.input_handler().await;
    // }
}

struct UserInputListener {
    ui_app: Arc<Mutex<UIApp<'static>>>,
}

impl UserInputListener {
    pub fn new(ui_app: Arc<Mutex<UIApp<'static>>>) -> Self {
        Self { ui_app }
    }

    pub fn run(&mut self) {
        // let event = read().unwrap();
        // let input: Input = event.clone().into();
        // loop {
        //     match read().unwrap().into() {
        //         Input { key: Key::Esc, .. } => break,
        //         Input {
        //             key: Key::Char('m'),
        //             ctrl: true,
        //             ..
        //         }
        //         | Input {
        //             key: Key::Enter, ..
        //         } => {}
        //         input => {
        //             let mut ui_app = self.ui_app.lock().unwrap();
        //             ui_app.input_area.input(input);
        //         }
        //     }
        // }
    }
}