use crate::kv::{KVCommand, KeyValue};
use sled::Db;

pub struct Database {
    sled: Db,
}

impl Database {
    pub fn new(path: &str) -> Self {
        Self {
            sled: sled::open(path).unwrap(),
        }
    }

    pub fn handle_command(&self, command: KVCommand) -> Option<String> {
        match command {
            KVCommand::Put(KeyValue { key, value }) => {
                self.put(&key, &value);
                None
            }
            KVCommand::Delete(key) => {
                self.delete(&key);
                None
            }
            KVCommand::Get(key) => self.get(key.as_str()),
        }
    }

    fn get(&self, key: &str) -> Option<String> {
        match self.sled.get(key.as_bytes()) {
            Ok(Some(value)) => {
                let value = String::from_utf8(value.as_ref().into()).unwrap();
                Some(value)
            }
            Ok(None) => None,
            Err(e) => panic!("failed to get value: {}", e),
        }
    }

    fn put(&self, key: &str, value: &str) {
        match self.sled.insert(key, value) {
            Ok(_) => {}
            Err(e) => panic!("failed to put value: {}", e),
        }
    }

    fn delete(&self, key: &str) {
        match self.sled.remove(key.as_bytes()) {
            Ok(_) => {}
            Err(e) => panic!("failed to delete value: {}", e),
        }
    }
}
