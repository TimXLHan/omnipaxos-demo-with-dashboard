use crate::messages::coordinator::CDMessage;
use crate::messages::ui::UIMessage;

pub mod coordinator {
    use crate::coordinator::KeyValue;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone)]
    pub enum CDMessage {
        StartBatchingPropose(u64),
        Initialize, // Launch to initialize the application
        KVCommand(KVCommand, Option<u64>),
        SetConnection(u64, Option<u64>, bool),
        OmnipaxosNodeCrashed(u64),
        OmnipaxosNodeJoined(u64),
        NewRound(u64, Option<Round>),
        Scenario(String),
    }

    /// Same as in KV demo
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum KVCommand {
        Put(KeyValue),
        Delete(String),
        Get(String),
    }

    #[derive(Clone, Copy, Eq, Debug, Ord, PartialOrd, PartialEq, Serialize, Deserialize)]
    pub struct Round {
        pub round_num: u32,
        pub leader: u64,
    }

    /// Same as in KV demo
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum APIResponse {
        Decided(u64),
        Get(String, Option<String>),
        NewRound(Option<Round>),
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub(crate) enum Message {
        APIRequest(KVCommand),
        APIResponse(APIResponse, u64),
    }
}

pub mod ui {
    use super::coordinator::APIResponse;
    use crate::coordinator::NetworkState;

    #[derive(Debug, Clone)]
    pub enum UIMessage {
        ClearConsole,
        Initialize, // Launch to initialize the application
        UpdateUi,
        OmnipaxosResponse(APIResponse, u64),
        OmnipaxosNetworkUpdate(NetworkState),
        OmnipaxosNodeCrashed(u64),
        ClusterUnreachable,
        NoSuchNode(u64, Vec<u64>),
        ProposalStatus(u64),
        #[allow(dead_code)]
        Debug(String),
        Exit,
    }
}

#[derive(Debug, Clone)]
pub enum IOMessage {
    CDMessage(CDMessage),
    UIMessage(UIMessage),
}
