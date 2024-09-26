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
    pub last_loop_cover: HashMap<NodeID, SystemTime>,
    pub last_drop: SystemTime,
    pub last_payload: SystemTime,
}

impl LoopixStorage {
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string::<LoopixStorageSave>(&LoopixStorageSave::V1(self.clone()))
    }
}


impl Default for LoopixStorage {
    fn default() -> Self {
        LoopixStorage {
            last_loop_cover: HashMap::new(),
            last_drop: SystemTime::now(),
            last_payload: SystemTime::now(),
        }
    }
}

// //////////////////////// Core //////////////////////////////// ////////////////////////
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LoopixCore {
    pub storage: LoopixStorage,
    pub config: LoopixConfig,
}

impl LoopixCore {
    pub fn new(storage: LoopixStorage, config: LoopixConfig) -> Self {
        Self { storage, config }
    }

    pub fn get_config(&self) -> &LoopixConfig {
        &self.config
    }

    pub fn get_storage(&self) -> &LoopixStorage {
        &self.storage
    }
}

pub trait NodeBehavior {
    fn send_loop_traffic(&self, node_id: NodeID);
    fn send_drop_traffic(&self, node_id: NodeID);
    fn send_payload_traffic(&self, node_id: NodeID);

    fn get_node_type(&self) -> &'static str {
        "Generic Node"
    }
}


// TODO circular imports?
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum LoopixNode {
    Provider(Provider),
    Client(Client),
    Mixnode(Mixnode),
}

impl LoopixNode {
    pub fn get_config(&self) -> &LoopixConfig {
        match self {
            LoopixNode::Provider(p) => &p.core.config,
            LoopixNode::Client(c) => &c.core.config,
            LoopixNode::Mixnode(m) => &m.core.config,
        }
    }

}

