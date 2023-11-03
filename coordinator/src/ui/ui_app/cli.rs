use std::fmt;
use std::str::Split;

use crate::coordinator::KeyValue;
use crate::messages::coordinator::{CDMessage, KVCommand};
use crate::messages::ui::UIMessage;
use crate::messages::IOMessage;

use tokio::sync::mpsc::Sender;

const INVALID_COMMAND: &str = "Valid commands are put/get/delete/connection/batch/scenario/clear";
const INVALID_DELETE: &str = "Invalid command, format is: delete <key-to-delete> [<node-id>]";
const INVALID_GET: &str = "Invalid command, format is: get <key-to-get> [<node-id>]";
const INVALID_PUT: &str = "Invalid command, format is: put <key> <value> [<node-id>]";
const INVALID_CONNECTION: &str =
    "Invalid command, format is: connection <node-id> [<another-node-id>] <true/false>";
const INVALID_CONNECTION_ARG1: &str = "Invalid command: first connection argument must be a number";
const INVALID_CONNECTION_ARG2: &str =
    "Invalid command: second connection argument must be a number";
const INVALID_CONNECTION_ARG3: &str = "Invalid command: third connection argument must be a bool";
const INVALID_BATCH: &str = "Invalid command, format is: batch <number-of-proposals>";
const INVALID_BATCH_ARG1: &str = "Invalid command: first batch argument must be a number";
const INVALID_SCENARIO: &str =
    "Invalid command, format is: scenario <restore/qloss/constrained/chained>";

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
            Ok((io_message, out)) => {
                self.io_sender.send(io_message).await.unwrap();
                out
            }
            Err(err) => {
                format!("{err}")
            }
        }
    }
}

struct ParseCommandError(String);
impl fmt::Display for ParseCommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn parse_command_proposer_and_string(
    mut words: Split<char>,
    error_msg: String,
) -> Result<(Option<u64>, String), ParseCommandError> {
    let pid = words.next();
    match pid {
        Some(pid) => {
            let pid = pid.parse();
            if pid.is_err() {
                return Err(ParseCommandError(error_msg));
            }
            let pid = pid.unwrap();
            Ok((Some(pid), format!("node {}", pid)))
        }
        None => Ok((None, "leader".to_string())),
    }
}

fn parse_command(line: String) -> Result<(IOMessage, String), ParseCommandError> {
    let mut words = line.trim().split(' ');
    let command_type = words
        .next()
        .ok_or(ParseCommandError(INVALID_COMMAND.to_string()))?;

    let command_output = match command_type {
        "clear" => (
            IOMessage::UIMessage(UIMessage::ClearConsole),
            "clear".to_string(),
        ),
        "delete" => {
            let value = words
                .next()
                .ok_or(ParseCommandError(INVALID_DELETE.to_string()))?;
            let (proposer, s) =
                parse_command_proposer_and_string(words, INVALID_DELETE.to_string())?;
            let output = format!("Delete key: {value} at {s}").to_string();
            let msg = IOMessage::CDMessage(CDMessage::KVCommand(
                KVCommand::Delete(value.to_string()),
                proposer,
            ));
            (msg, output)
        }
        "get" => {
            let value = words
                .next()
                .ok_or(ParseCommandError(INVALID_GET.to_string()))?;
            let (proposer, s) = parse_command_proposer_and_string(words, INVALID_GET.to_string())?;
            let output = format!("Get key: {value} from {s}").to_string();
            let msg = IOMessage::CDMessage(CDMessage::KVCommand(
                KVCommand::Get(value.to_string()),
                proposer,
            ));
            (msg, output)
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
            let (proposer, s) = parse_command_proposer_and_string(words, INVALID_PUT.to_string())?;
            let output = format!("Put key: {key}, value: {value} at {s}");
            let msg = IOMessage::CDMessage(CDMessage::KVCommand(
                KVCommand::Put(KeyValue { key, value }),
                proposer,
            ));
            (msg, output)
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
            let s = match to {
                Some(to) => format!("node {}", to),
                None => "all nodes".to_string(),
            };
            let output = if connection_status {
                format!("Connecting node {from} to {s}")
            } else {
                format!("Disconnecting node {from} from {s}")
            };
            let msg = IOMessage::CDMessage(CDMessage::SetConnection(from, to, connection_status));
            (msg, output)
        }
        "batch" => {
            let num_proposals = words
                .next()
                .ok_or(ParseCommandError(INVALID_BATCH.to_string()))?
                .parse::<u64>()
                .map_err(|_| ParseCommandError(INVALID_BATCH_ARG1.to_string()))?;
            let msg = IOMessage::CDMessage(CDMessage::StartBatchingPropose(num_proposals));
            let output = format!("Batching {} put operations", num_proposals);
            (msg, output)
        }
        "scenario" => {
            let scenario_type = words
                .next()
                .ok_or(ParseCommandError(INVALID_SCENARIO.to_string()))?;
            let output = match scenario_type {
                "qloss" => format!("Creating quorum-loss scenario"),
                "constrained" => format!("Creating constrained election scenario"),
                "chained" => format!("Creating chained scenario"),
                "restore" => format!("Restoring all connections"),
                _ => return Err(ParseCommandError(INVALID_SCENARIO.to_string())),
            };
            let msg = IOMessage::CDMessage(CDMessage::Scenario(scenario_type.to_string()));
            (msg, output)
        }
        _ => Err(ParseCommandError(INVALID_COMMAND.to_string()))?,
    };
    Ok(command_output)
}
