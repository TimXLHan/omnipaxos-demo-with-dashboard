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
use crate::ui::ui_app::{UIApp};
use crate::utils::{UI_MAX_DECIDED_BARS, UI_TICK_RATE};

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
            UIMessage::ClearConsole => {
                self.ui_app.lock().await.clear_logs();
                self.update_ui().await;
            }
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
                self.ui_app.lock().await.set_network_state(network_statue);
                self.update_ui().await;
            }
            UIMessage::OmnipaxosResponse(response) => match response {
                APIResponse::Decided(idx) => {
                    let mut ui_app = self.ui_app.lock().await;
                    ui_app.progress.finished = idx - ui_app.progress.starting_idx;

                    ui_app.decided_idx = idx;
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
            UIMessage::ProposalStatus(total_batched_num) => {
                let mut ui_app = self.ui_app.lock().await;
                // Append log
                if total_batched_num != 0 {
                    // Start new batch
                    if !ui_app.progress.is_ongoing {
                        ui_app.progress.is_ongoing = true;
                        ui_app.progress.total = total_batched_num;
                        ui_app.progress.starting_idx = ui_app.decided_idx;
                        ui_app.progress.finished = 0;
                    } else {
                        ui_app.progress.total = total_batched_num;
                    }
                } else {
                    ui_app.progress.is_ongoing = false;
                }
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
        let mut counter: f64 = 0.0;
        loop {
            tokio::select! {
                _ = ui_interval.tick() => {
                    counter += 1.0;
                    let mut ui_app = self.ui_app.lock().await;
                    let num_decided = (ui_app.decided_idx - last_decided_idx) as f64;
                    let round = if num_decided as u64 == 0 {
                        " ".to_string()
                    } else {
                        match ui_app.network_state.max_round {
                        Some(round) => format!("{}", round.round_num),
                        None => "0".to_string(),
                    }
                    };
                    ui_app.decided_data.insert(0, (round, num_decided as u64));
                    if ui_app.decided_data.len() > UI_MAX_DECIDED_BARS {
                        ui_app.decided_data.pop();
                    }
                    last_decided_idx = ui_app.decided_idx;
                    if counter * UI_TICK_RATE.as_secs_f64() >= 1.0 {
                        counter = 0.0;
                        ui_app.throughput = num_decided/ UI_TICK_RATE.as_secs_f64();
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
                        let mut ui_app = self.ui_app.lock().await;
                        ui_app.scroll -= 1;
                        self.io_sender
                            .send(IOMessage::UIMessage(UIMessage::UpdateUi))
                            .await
                            .unwrap();
                    }
                    Input { key: Key::Down, .. } => {
                        let mut ui_app = self.ui_app.lock().await;
                        ui_app.scroll = (ui_app.scroll + 1).min(0);
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
                        ui_app.scroll = 0;
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
