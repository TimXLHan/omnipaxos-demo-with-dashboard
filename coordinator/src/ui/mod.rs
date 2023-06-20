use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::{Frame, Terminal};
use crossterm::event::{Event, read};
use tui_textarea::{Input, Key};
use std::io::stdout;
use std::sync::Arc;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen
};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};

use crate::messages::{IOMessage, ui::UIMessage};
use crate::ui::ui_app::cli::CLIHandler;
use crate::ui::ui_app::render::render;
use crate::ui::ui_app::UIApp;

mod ui_app;

pub struct UI {
    ui_app: Arc<Mutex<UIApp<'static>>>,
    terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    io_sender: Sender<IOMessage>,
}

impl UI {
    pub(crate) fn new(io_sender: Sender<IOMessage>) -> Self {
        // Configure Crossterm backend for tui
        let stdout = stdout();
        enable_raw_mode().unwrap();
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).unwrap();
        Self {
            io_sender: io_sender.clone(),
            ui_app: Arc::new(Mutex::new(UIApp::new(io_sender))),
            terminal,
        }
    }

    pub(crate) async fn handle(&mut self, m: UIMessage) {
        match m {
            UIMessage::Initialize => {
                self.start().await;
            }
            UIMessage::UpdateUi => {
                self.update_ui().await;
            }
            UIMessage::Exit => {
                disable_raw_mode().unwrap();
                crossterm::execute!(
                    self.terminal.backend_mut(),
                    LeaveAlternateScreen,
                    DisableMouseCapture
                ).unwrap();
                self.terminal.clear().unwrap();
                self.terminal.show_cursor().unwrap();
                std::process::exit(0);
            }
            UIMessage::OmnipaxosNetworkUpdate(network_statue) => {
                self.ui_app.lock().await.network_state = network_statue;
                self.update_ui().await;
            }
            _ => println!("not implemented"),
        }
    }

    pub async fn start(&mut self) {
        self.terminal.clear().unwrap();
        self.terminal.hide_cursor().unwrap();
        let ui_app = self.ui_app.clone();
        let io_sender = self.io_sender.clone();
        tokio::spawn(async move {
            UserInputListener::new(ui_app, io_sender).run().await;
        });
        self.io_sender.send(IOMessage::UIMessage(UIMessage::UpdateUi)).await.unwrap();
        // self.update_ui().await;
    }

    async fn update_ui(&mut self) {
        let ui_app = self.ui_app.lock().await;
        self.terminal.draw(|rect| render(rect, &ui_app)).unwrap();
    }
}

struct UserInputListener {
    ui_app: Arc<Mutex<UIApp<'static>>>,
    io_sender: Sender<IOMessage>,
    cli_handler: CLIHandler
}

impl UserInputListener {
    pub fn new(ui_app: Arc<Mutex<UIApp<'static>>>, io_sender: Sender<IOMessage>) -> Self {
        Self {
            ui_app,
            cli_handler: CLIHandler::new(io_sender.clone()),
            io_sender,
        }
    }

    pub async fn run(&mut self) {
        loop {
            match read().unwrap().into() {
                Input { key: Key::Esc, .. } => {
                    self.io_sender.send(IOMessage::UIMessage(UIMessage::Exit)).await.unwrap();
                    break;
                }
                Input { key: Key::Up, .. } => {
                    self.ui_app.lock().await.scroll -= 1;
                    self.io_sender.send(IOMessage::UIMessage(UIMessage::UpdateUi)).await.unwrap();
                }
                Input { key: Key::Down, .. } => {
                    let mut ui_app = self.ui_app.lock().await;
                    let scroll = ui_app.scroll;
                    ui_app.scroll = (scroll + 1).min(0);
                    self.io_sender.send(IOMessage::UIMessage(UIMessage::UpdateUi)).await.unwrap();
                }
                Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                } => {
                    self.io_sender.send(IOMessage::UIMessage(UIMessage::Exit)).await.unwrap();
                    break;
                }
                Input {
                    key: Key::Enter, ..
                } => {
                    let mut ui_app = self.ui_app.lock().await;
                    let log = ui_app.input_area.lines()[0].clone();
                    let out = self.cli_handler.handle_user_input(log).await;
                    if !out.is_empty() {
                        ui_app.append_log(out);
                    }
                    ui_app.input_area.delete_line_by_head();
                    ui_app.input_area.delete_line_by_end();
                    self.io_sender.send(IOMessage::UIMessage(UIMessage::UpdateUi)).await.unwrap();
                }
                input => {
                    self.ui_app.lock().await.input_area.input(input);
                    self.io_sender.send(IOMessage::UIMessage(UIMessage::UpdateUi)).await.unwrap();
                }
            }
        }
    }
}
