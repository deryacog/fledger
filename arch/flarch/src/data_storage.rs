use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use thiserror::Error;

#[cfg(feature = "node")]
mod node;
#[cfg(feature = "node")]
pub use node::*;
#[cfg(all(feature = "wasm", not(feature = "node")))]
mod wasm;
#[cfg(all(feature = "wasm", not(feature = "node")))]
pub use wasm::*;
#[cfg(not(feature = "wasm"))]
mod libc;
#[cfg(not(feature = "wasm"))]
pub use libc::*;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("From the underlying storage: {0}")]
    Underlying(String),
}

/// The DataStorage trait allows access to a persistent storage.
pub trait DataStorage {
    fn get(&self, key: &str) -> Result<String, StorageError>;

    fn set(&mut self, key: &str, value: &str) -> Result<(), StorageError>;

    fn remove(&mut self, key: &str) -> Result<(), StorageError>;

    fn clone(&self) -> Box<dyn DataStorage>;
}

/// A temporary DataStorage.
pub struct TempDS {
    kvs: Arc<Mutex<HashMap<String, String>>>,
}

impl TempDS {
    pub fn new(
    ) -> Box<Self> {
        Box::new(Self { kvs: Arc::new(Mutex::new(HashMap::new()))})
    }
}

impl DataStorage for TempDS {
    fn get(&self, key: &str) -> Result<String, StorageError> {
        let mut kvs = self
            .kvs
            .try_lock()
            .map_err(|e| StorageError::Underlying(e.to_string()))?;
        if let Some(kvs_entry) = kvs.get_mut(key) {
            Ok(kvs_entry.clone())
        } else {
            Ok("".to_string())
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        let mut kvs = self
            .kvs
            .try_lock()
            .map_err(|e| StorageError::Underlying(e.to_string()))?;
        kvs.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn remove(&mut self, key: &str) -> Result<(), StorageError> {
        let mut kvs = self
            .kvs
            .try_lock()
            .map_err(|e| StorageError::Underlying(e.to_string()))?;
        kvs.remove(key);
        Ok(())
    }

    fn clone(&self) -> Box<dyn DataStorage>{
        Box::new(Self{kvs: Arc::clone(&self.kvs)})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage() -> Result<(), Box<dyn std::error::Error>>{
        let dsb = TempDSB::new();
        let mut ds = dsb.get("one");
        ds.set("two", "three")?;

        let dsb2 = dsb.clone();
        let ds2 = dsb2.get("one");
        assert_eq!("three", ds2.get("two")?);
        Ok(())
    }
}