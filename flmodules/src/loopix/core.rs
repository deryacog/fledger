use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime};
use super::Provider;
use super::Client;
use super::Mixnode;

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
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LoopixCore {
    pub storage: LoopixStorage,
    pub config: LoopixConfig,
    pub key_pair: (PublicKey, PrivateKey), // TODO: definitions of PublicKey and PrivateKey
}

impl LoopixCore {
    pub fn new(storage: LoopixStorage, config: LoopixConfig, key_pair: Option<(PublicKey, PrivateKey)>) -> Self {
        let key_pair = key_pair.unwrap_or_else(generate_key_pair); // TODO generate key pair
        Self { storage, config, key_pair }
    }

    pub fn get_config(&self) -> &LoopixConfig {
        &self.config
    }

    pub fn get_storage(&self) -> &LoopixStorage {
        &self.storage
    }

    pub fn get_key_pair(&self) -> &(PublicKey, PrivateKey) {
        &self.key_pair
    }

}

pub trait NodeBehavior {
    fn process_loopix_message(&self, message: Message);
}
