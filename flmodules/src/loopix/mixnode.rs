use super::core::{LoopixCore, LoopixConfig, LoopixStorage, NodeBehavior};
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use crate::loopix::messages::Message;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Mixnode {
    pub core: LoopixCore,
}

pub trait MixnodeInterface {
    fn new() -> Self;

    fn create_loop_message(&self, node_id: NodeID) {
        // Default implementation
    }

    fn create_drop_message(&self, node_id: NodeID) {
        // Default implementation
    }

    // TODO some kind of send queue
}

impl MixnodeInterface for Mixnode {
    fn new() -> Self {
        // TODO: Generate key pair
        Self {
            core: LoopixCore::new(
                LoopixStorage::default(),
                LoopixConfig {
                    lambda_loop: 2.0,
                    lambda_drop: 0.0,
                    lambda_payload: 0.0,
                    path_length: 0,
                    mean_delay: 0.001,
                    lambda_loop_mix: 500.0,
                },
            ),
        }
    }

}

impl NodeBehavior for Mixnode {
    fn process_loopix_message(&self, message: Message) {
        // basically routing
        // TODO: Implement
    }
    // method to sendd the messages
}
