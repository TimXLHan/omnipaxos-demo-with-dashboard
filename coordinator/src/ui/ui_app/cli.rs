use std::fmt;

use crate::coordinator::KeyValue;
use crate::messages::coordinator::{CDMessage, KVCommand};
use crate::messages::IOMessage;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc::Sender;

pub struct CLIHandler {
    io_sender: Sender<IOMessage>,
}

impl CLIHandler {
    pub fn new(io_sender: Sender<IOMessage>) -> Self {
        Self { io_sender }
    }

    // Handle the user input and return a output string (can be empty) to be displayed on the output area.
    pub async fn handle_user_input(&mut self, input: String) -> String {
        match parse_command(input) {
            Ok(io_message) => {
                let out = format!("{:?}", &io_message);
                self.io_sender.send(io_message).await.unwrap();
                out
            }
            Err(err) => {
                //let now: String = Utc::now().format("%F %T> ").to_string();
                //now + &*input
                format!("{err}")
            }
        }
    }
}

const INVALID_COMMAND: &str =
    "Invalid command: valid commands are put/get/delete/connection/batch/scenario";
const INVALID_DELETE: &str = "Invalid command, format is: delete <key-to-delete>";
const INVALID_GET: &str = "Invalid command, format is: get <key-to-get>";
const INVALID_PUT: &str = "Invalid command, format is: put <key> <value>";
const INVALID_CONNECTION: &str =
    "Invalid command, format is: connection <node-id> <node-id> <true/false>";
const INVALID_CONNECTION_ARG1: &str = "Invalid command: first connection argument must be a number";
const INVALID_CONNECTION_ARG2: &str =
    "Invalid command: second connection argument must be a number";
const INVALID_CONNECTION_ARG3: &str = "Invalid command: third connection argument must be a bool";
const INVALID_BATCH: &str = "Invalid command, format is: batch <number-of-proposals>";
const INVALID_BATCH_ARG1: &str = "Invalid command: first batch argument must be a number";
const INVALID_SCENARIO: &str =
    "Invalid command, format is: scenario <restore/qloss/constrained/chained>";

struct ParseCommandError(String);
impl fmt::Display for ParseCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
fn parse_command(line: String) -> Result<IOMessage, ParseCommandError> {
    let mut words = line.trim().split(" ");
    let command_type = words
        .next()
        .ok_or(ParseCommandError(INVALID_COMMAND.to_string()))?;

    let command = match command_type {
        "delete" => {
            let value = words
                .next()
                .ok_or(ParseCommandError(INVALID_DELETE.to_string()))?;
            IOMessage::CDMessage(CDMessage::KVCommand(KVCommand::Delete(value.to_string())))
        }
        "get" => {
            let value = words
                .next()
                .ok_or(ParseCommandError(INVALID_GET.to_string()))?;
            IOMessage::CDMessage(CDMessage::KVCommand(KVCommand::Get(value.to_string())))
        }
        "put" => {
            let key = words
                .next()
                .ok_or(ParseCommandError(INVALID_PUT.to_string()))?
                .to_string();
            let value = words
                .next()
                .ok_or(ParseCommandError(INVALID_PUT.to_string()))?
                .to_string();
            IOMessage::CDMessage(CDMessage::KVCommand(KVCommand::Put(KeyValue {
                key,
                value,
            })))
        }
        "connection" => {
            let args = words.collect::<Vec<&str>>();
            if args.len() < 2 {
                Err(ParseCommandError(INVALID_CONNECTION.to_string()))?
            }
            let (from, to, connection_status) = if args.len() == 2 {
                let from = args[0]
                    .parse::<u64>()
                    .map_err(|_| ParseCommandError(INVALID_CONNECTION_ARG1.to_string()))?;
                let connection_status = args[1]
                    .parse::<bool>()
                    .map_err(|_| ParseCommandError(INVALID_CONNECTION_ARG3.to_string()))?;
                (from, None, connection_status)
            } else {
                let from = args[0]
                    .parse::<u64>()
                    .map_err(|_| ParseCommandError(INVALID_CONNECTION_ARG1.to_string()))?;
                let to = args[1]
                    .parse::<u64>()
                    .map_err(|_| ParseCommandError(INVALID_CONNECTION_ARG2.to_string()))?;
                let connection_status = args[2]
                    .parse::<bool>()
                    .map_err(|_| ParseCommandError(INVALID_CONNECTION_ARG3.to_string()))?;
                (from, Some(to), connection_status)
            };
            IOMessage::CDMessage(CDMessage::SetConnection(from, to, connection_status))
        }
        "batch" => {
            let num_proposals = words
                .next()
                .ok_or(ParseCommandError(INVALID_BATCH.to_string()))?
                .parse::<u64>()
                .map_err(|_| ParseCommandError(INVALID_BATCH_ARG1.to_string()))?;
            IOMessage::CDMessage(CDMessage::StartBatchingPropose(num_proposals))
        }
        "scenario" => {
            let scenario_type = words
                .next()
                .ok_or(ParseCommandError(INVALID_SCENARIO.to_string()))?;
            match scenario_type {
                "qloss" => (),
                "constrained" => (),
                "chained" => (),
                "restore" => (),
                _ => return Err(ParseCommandError(INVALID_SCENARIO.to_string())),
            }
            IOMessage::CDMessage(CDMessage::Scenario(scenario_type.to_string()))
        }
        _ => Err(ParseCommandError(INVALID_COMMAND.to_string()))?,
    };
    Ok(command)
}
