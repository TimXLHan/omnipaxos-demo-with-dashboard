use omnipaxos::macros::Entry;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

/// Same as in network actor
#[derive(Debug, Clone, Serialize, Deserialize, Entry)]
pub enum KVCommand {
    Put(KeyValue),
    Delete(String),
    Get(String),
}
