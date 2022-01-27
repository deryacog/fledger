use wasm_bindgen::prelude::*;
use web_sys::window;

use types::data_storage::{DataStorage, DataStorageBase, StorageError};

pub struct LocalStorageBase {}

impl DataStorageBase for LocalStorageBase {
    fn get(&self, base_str: &str) -> Box<dyn DataStorage> {
        let base = if base_str.is_empty() {
            "".to_string()
        } else {
            base_str.to_string() + "_"
        };
        Box::new(LocalStorage { base })
    }
    fn clone(&self) -> Box<dyn DataStorageBase> {
        Box::new(LocalStorageBase {})
    }
}

pub struct LocalStorage {
    base: String,
}

impl DataStorage for LocalStorage {
    fn get(&self, key: &str) -> Result<String, StorageError> {
        let key_entry = format!("{}{}", self.base, key);
        Ok(window()
            .unwrap()
            .local_storage()
            .map_err(|e| StorageError::Underlying(e.as_string().unwrap()))?
            .unwrap()
            .get(&key_entry)
            .map_err(|e| StorageError::Underlying(e.as_string().unwrap()))?
            .unwrap_or_else(|| "".to_string()))
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        let key_entry = format!("{}{}", self.base, key);
        window()
            .unwrap()
            .local_storage()
            .map_err(|e| StorageError::Underlying(e.as_string().unwrap()))?
            .unwrap()
            .set(&key_entry, value)
            .map_err(|e| StorageError::Underlying(e.as_string().unwrap()))
    }

    fn remove(&mut self, key: &str) -> Result<(), StorageError> {
        let key_entry = format!("{}{}", self.base, key);
        window()
            .unwrap()
            .local_storage()
            .map_err(|e| StorageError::Underlying(e.as_string().unwrap()))?
            .unwrap()
            .remove_item(&key_entry)
            .map_err(|e| StorageError::Underlying(e.as_string().unwrap()))
    }
}

#[cfg_attr(
    feature = "node",
    wasm_bindgen(
        inline_js = "module.exports.wait_ms = function(ms){ return new Promise((r) => setTimeout(r, ms));}"
    )
)]
#[cfg_attr(
    not(feature = "node"),
    wasm_bindgen(
        inline_js = "export function wait_ms(ms){ return new Promise((r) => setTimeout(r, ms));}"
    )
)]
extern "C" {
    pub async fn wait_ms(ms: u32);
}
