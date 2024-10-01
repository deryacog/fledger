use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use x25519_dalek::{PublicKey, StaticSecret};
use crate::loopix::messages::Message;

// //////////////////////// Config //////////////////////////////// ////////////////////////
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LoopixConfig {
    pub lambda_loop: f64,       // Loop traffic rate (user)
    pub lambda_drop: f64,       // Drop cover traffic rate (user)
    pub lambda_payload: f64,    // Payload traffic rate (user)
    pub path_length: i32,       // Path length (number of mix nodes in the route)
    pub mean_delay: f64,        // Mean delay at each mix node (in seconds)
    pub lambda_loop_mix: f64,   // Loop traffic rate (mix)
}

impl Default for LoopixConfig {
    fn default() -> Self {
        LoopixConfig {
            lambda_loop: 10.0,
            lambda_drop: 10.0,
            lambda_payload: 10.0,
            path_length: 3,
            mean_delay: 0.001,
            lambda_loop_mix: 10.0,
        }
    }
}

// //////////////////////// Storage //////////////////////////////// ////////////////////////
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum LoopixStorageSave {
    V1(LoopixStorage),
}

impl LoopixStorageSave {
    pub fn from_str(data: &str) -> Result<LoopixStorage, serde_yaml::Error> {
        return Ok(serde_yaml::from_str::<LoopixStorageSave>(data)?.to_latest());
    }

    fn to_latest(self) -> LoopixStorage {
        match self {
            LoopixStorageSave::V1(es) => es,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LoopixStorage {
    pub last_loop_cover: SystemTime,
    pub last_drop: SystemTime,
    pub last_payload: SystemTime,
    pub last_pull: SystemTime,
    pub last_real: SystemTime,
}

impl LoopixStorage {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string::<LoopixStorageSave>(&LoopixStorageSave::V1(self.clone()))
    }
}

impl Default for LoopixStorage {
    fn default() -> Self {
        LoopixStorage {
            last_loop_cover: SystemTime::now(),
            last_drop: SystemTime::now(),
            last_payload: SystemTime::now(),
            last_pull: SystemTime::now(),
            last_real: SystemTime::now(),
        }
    }
}

// //////////////////////// Core ////////////////////////////////////////////////////////
// #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LoopixCore {
    pub storage: LoopixStorage,
    pub config: LoopixConfig,
    key_pair: (PublicKey, StaticSecret),
}

impl LoopixCore {
    pub fn new(storage: LoopixStorage, config: LoopixConfig) -> Self {
        let key_pair = Self::generate_key_pair();
        Self { storage, config, key_pair }
    }

    pub fn get_config(&self) -> &LoopixConfig {
        &self.config
    }

    pub fn get_storage(&self) -> &LoopixStorage {
        &self.storage
    }

    pub fn get_public_key(&self) -> &PublicKey {
        &self.key_pair.0
    }

    fn generate_key_pair() -> (PublicKey, StaticSecret) {
        let rng = rand::thread_rng();
        let private_key = StaticSecret::random_from_rng(rng);
        let public_key = PublicKey::from(&private_key);
        (public_key, private_key)
    }
}

pub trait NodeBehavior {
    fn process_loopix_message(&self, message: Message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loopix_config_default() {
        let config = LoopixConfig::default();
        assert_eq!(config.lambda_loop, 10.0);
        assert_eq!(config.lambda_drop, 10.0);
        assert_eq!(config.lambda_payload, 10.0);
        assert_eq!(config.path_length, 3);
        assert_eq!(config.mean_delay, 0.001);
        assert_eq!(config.lambda_loop_mix, 10.0);
    }

    #[test]
    fn test_loopix_storage_default() {
        let storage = LoopixStorage::default();
        let now = SystemTime::now();
        assert!(storage.last_loop_cover <= now);
        assert!(storage.last_drop <= now);
        assert!(storage.last_payload <= now);
        assert!(storage.last_pull <= now);
        assert!(storage.last_real <= now);
    }

    #[test]
    fn test_loopix_storage_serialization() {
        let storage = LoopixStorage::default();
        let yaml = storage.to_yaml().unwrap();
        let deserialized = LoopixStorageSave::from_str(&yaml).unwrap();
        assert_eq!(storage, deserialized);
    }

    #[test]
    fn test_loopix_core_new() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage.clone(), config.clone());
        
        assert_eq!(core.storage, storage);
        assert_eq!(core.config, config);
        
        // Generate a new key pair for comparison
        let (new_public_key, _) = LoopixCore::generate_key_pair();
        
        // Check that the generated public key is different
        // This test may theoretically fail with an extremely low probability
        assert_ne!(core.get_public_key(), &new_public_key);
    }

    #[test]
    fn test_loopix_core_getters() {
        let storage = LoopixStorage::default();
        let config = LoopixConfig::default();
        let core = LoopixCore::new(storage.clone(), config.clone());
        
        assert_eq!(core.get_config(), &config);
        assert_eq!(core.get_storage(), &storage);
        assert_eq!(core.get_public_key(), &core.key_pair.0);
    }

    #[test]
    fn test_loopix_storage_save_from_str() {
        let storage = LoopixStorage::default();
        let yaml = storage.to_yaml().unwrap();
        let deserialized = LoopixStorageSave::from_str(&yaml).unwrap();
        assert_eq!(storage, deserialized);
    }

    #[test]
    fn test_message_creation() {
        let (sender_public, _) = LoopixCore::generate_key_pair();
        let (recipient_public, _) = LoopixCore::generate_key_pair();
        let content = b"Test message".to_vec();
        let timestamp = SystemTime::now();

        let message = Message {
            sender: sender_public,
            recipient: recipient_public,
            content: content.clone(),
            timestamp,
        };

        assert_eq!(message.sender, sender_public);
        assert_eq!(message.recipient, recipient_public);
        assert_eq!(message.content, content);
        assert_eq!(message.timestamp, timestamp);
    }
}
