use super::core::{LoopixCore, LoopixConfig, LoopixStorage, NodeBehavior};
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Client {
    pub core: LoopixCore,
}

impl Client {
    pub fn new() -> Self {
        Self {
            core: LoopixCore::new(
                LoopixStorage::default(),
                LoopixConfig {
                    lambda_loop: 2.0,
                    lambda_drop: 500.0,
                    lambda_payload: 2.0,
                    path_length: 3,
                    mean_delay: 0.001,
                    lambda_loop_mix: 0.0,
                },
            ),
        }
    }
}

impl NodeBehavior for Client {
    fn send_loop_traffic(&self, _node_id: NodeID) { /* TODO: Implement */ }
    fn send_drop_traffic(&self, _node_id: NodeID) { /* TODO: Implement */ }
    fn send_payload_traffic(&self, _node_id: NodeID) { /* TODO: Implement */ }

    fn get_node_type(&self) -> &'static str {
        "Client"
    }
}