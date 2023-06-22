use crate::messages::coordinator::CDMessage;
use crate::messages::ui::UIMessage;

pub mod coordinator {
    use crate::coordinator::KeyValue;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone)]
    pub enum CDMessage {
        Initialize, // Launch to initialize the application
        KVCommand(KVCommand),
        SetConnection(u64, u64, bool),
        OmnipaxosNodeCrashed(u64),
        OmnipaxosNodeJoined(u64),
    }

    /// Same as in KV demo
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum KVCommand {
        Put(KeyValue),
        Delete(String),
        Get(String),
    }

    /// Same as in KV demo
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum APIResponse {
        Decided(u64),
        Read(String, Option<String>),
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub(crate) enum Message {
        APICommand(KVCommand),
        APIResponse(APIResponse),
    }
}

pub mod ui {
    use super::coordinator::APIResponse;
    use crate::coordinator::NetworkState;

    #[derive(Debug, Clone)]
    pub enum UIMessage {
        Initialize, // Launch to initialize the application
        UpdateUi,
        OmnipaxosResponse(APIResponse),
        OmnipaxosNetworkUpdate(NetworkState),
        OmnipaxosNodeCrashed(u64),
        ClusterUnreachable,
        NoSuchNode(u64, Vec<u64>),
        Debug(String),
        Exit,
    }
}

#[derive(Debug, Clone)]
pub enum IOMessage {
    CDMessage(CDMessage),
    UIMessage(UIMessage),
}
