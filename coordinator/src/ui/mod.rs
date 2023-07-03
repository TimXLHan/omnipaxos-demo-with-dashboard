use crossterm::event::DisableMouseCapture;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tui_textarea::{Input, Key};

use std::io::stdout;
use std::sync::Arc;

use crate::messages::coordinator::APIResponse;
use crate::messages::{ui::UIMessage, IOMessage};
use crate::ui::ui_app::cli::CLIHandler;
use crate::ui::ui_app::render::render;
use crate::ui::ui_app::UIApp;
use crate::utils::{UI_MAX_THROUGHPUT_SIZE, UI_TICK_RATE};

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
        let terminal = Terminal::new(backend).unwrap();
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
                )
                .unwrap();
                self.terminal.clear().unwrap();
                self.terminal.show_cursor().unwrap();
                std::process::exit(0);
            }
            UIMessage::OmnipaxosNetworkUpdate(mut network_statue) => {
                network_statue.nodes.sort();
                network_statue.alive_nodes.sort();
                let msg = format!("New network state {:?}", network_statue);
                self.io_sender
                    .send(IOMessage::UIMessage(UIMessage::Debug(msg)))
                    .await
                    .unwrap();
                self.ui_app.lock().await.network_state = network_statue;
                self.update_ui().await;
            }
            UIMessage::OmnipaxosResponse(response) => match response {
                APIResponse::Decided(idx) => {
                    self.ui_app.lock().await.decided_idx = idx;
                }
                APIResponse::Get(key, value) => {
                    self.ui_app
                        .lock()
                        .await
                        .append_log(format!("The key: {key} has value: {:?}", value));
                    self.update_ui().await;
                }
                // Ignore this case. Will get notified in OmniPaxosNetworkUpdate instead
                APIResponse::NewRound(_) => (),
            },
            UIMessage::OmnipaxosNodeCrashed(id) => {
                self.ui_app
                    .lock()
                    .await
                    .append_log(format!("Lost connection to node {id}"));
                self.update_ui().await;
            }
            UIMessage::ClusterUnreachable => {
                self.ui_app
                    .lock()
                    .await
                    .append_log(format!("Couldn't reach cluster"));
                self.update_ui().await;
            }
            UIMessage::NoSuchNode(invalid_node_id, valid_node_ids) => {
                self.ui_app.lock().await.append_log(format!(
                    "Node {invalid_node_id} doesn't exists. Valid nodes are: {:?}",
                    valid_node_ids
                ));
                self.update_ui().await;
            }
            UIMessage::Debug(string) => {
                self.ui_app.lock().await.append_log(string);
                self.update_ui().await;
            }
        }
    }

    pub async fn start(&mut self) {
        self.terminal.clear().unwrap();
        self.terminal.hide_cursor().unwrap();
        let ui_app = self.ui_app.clone();
        // Run input listener
        let io_sender = self.io_sender.clone();
        tokio::spawn(async move {
            UserInputListener::new(ui_app, io_sender).run().await;
        });
        // Run ticker
        let io_sender = self.io_sender.clone();
        let ui_app = self.ui_app.clone();
        tokio::spawn(async move {
            let mut ticker = Ticker::new(io_sender, ui_app);
            ticker.run().await;
        });
        self.io_sender
            .send(IOMessage::UIMessage(UIMessage::UpdateUi))
            .await
            .unwrap();
    }

    async fn update_ui(&mut self) {
        let ui_app = self.ui_app.lock().await;
        self.terminal.draw(|rect| render(rect, &ui_app)).unwrap();
    }
}

struct Ticker {
    io_sender: Sender<IOMessage>,
    ui_app: Arc<Mutex<UIApp<'static>>>,
}

impl Ticker {
    pub fn new(io_sender: Sender<IOMessage>, ui_app: Arc<Mutex<UIApp<'static>>>) -> Self {
        Self { io_sender, ui_app }
    }

    pub async fn run(&mut self) {
        let mut ui_interval = tokio::time::interval(UI_TICK_RATE);
        let mut last_decided_idx: u64 = self.ui_app.lock().await.decided_idx;
        loop {
            tokio::select! {
                _ = ui_interval.tick() => {
                    {
                        let mut ui_app = self.ui_app.lock().await;
                        let throughput = (ui_app.decided_idx as f64 - last_decided_idx as f64).max(0.0) as f64 / (UI_TICK_RATE.as_millis() as f64 / 100.0) as f64;
                        ui_app.throughput_data.insert(0, ("CL".to_string(), throughput as u64));
                        if ui_app.throughput_data.len() > UI_MAX_THROUGHPUT_SIZE {
                            ui_app.throughput_data.pop();
                        }
                        last_decided_idx = ui_app.decided_idx;
                    }
                    self.io_sender.send(IOMessage::UIMessage(UIMessage::UpdateUi)).await.unwrap();
                }
            }
        }
    }
}

struct UserInputListener {
    ui_app: Arc<Mutex<UIApp<'static>>>,
    io_sender: Sender<IOMessage>,
    cli_handler: CLIHandler,
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
            if crossterm::event::poll(UI_TICK_RATE).unwrap() {
                match crossterm::event::read().unwrap().into() {
                    Input { key: Key::Esc, .. } => {
                        self.io_sender
                            .send(IOMessage::UIMessage(UIMessage::Exit))
                            .await
                            .unwrap();
                        break;
                    }
                    Input { key: Key::Up, .. } => {
                        self.ui_app.lock().await.scroll -= 1;
                        self.io_sender
                            .send(IOMessage::UIMessage(UIMessage::UpdateUi))
                            .await
                            .unwrap();
                    }
                    Input { key: Key::Down, .. } => {
                        let mut ui_app = self.ui_app.lock().await;
                        let scroll = ui_app.scroll;
                        ui_app.scroll = (scroll + 1).min(0);
                        self.io_sender
                            .send(IOMessage::UIMessage(UIMessage::UpdateUi))
                            .await
                            .unwrap();
                    }
                    Input {
                        key: Key::Char('c'),
                        ctrl: true,
                        ..
                    } => {
                        self.io_sender
                            .send(IOMessage::UIMessage(UIMessage::Exit))
                            .await
                            .unwrap();
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
                        self.io_sender
                            .send(IOMessage::UIMessage(UIMessage::UpdateUi))
                            .await
                            .unwrap();
                    }
                    input => {
                        self.ui_app.lock().await.input_area.input(input);
                        self.io_sender
                            .send(IOMessage::UIMessage(UIMessage::UpdateUi))
                            .await
                            .unwrap();
                    }
                }
            }
        }
    }
}
