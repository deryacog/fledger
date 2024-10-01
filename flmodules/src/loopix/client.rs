use super::core::{LoopixCore, LoopixConfig, LoopixStorage, NodeBehavior};
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use crate::loopix::messages::Message;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Client {
    pub core: LoopixCore,
    provider: Option<NodeID>,
    // mixnodes: Vec<NodeID>, // maybe?
}

pub trait ClientInterface {
    fn new() -> Self;

    fn register_provider(&mut self, provider: NodeID);
    fn get_provider(&self) -> Option<NodeID>;
    fn send_pull_request(&self);

    fn create_loop_message(&self);
    fn create_drop_message(&self);
    fn create_payload_message(&self, destination: NodeID);
    
    // TODO some kind of send queue
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
            provider: None,
        }
    }

    pub fn register_provider(&mut self, provider: NodeID) {
        self.provider = Some(provider);
        // TODO: Send registration message to provider
    }

    pub fn get_provider(&self) -> Option<NodeID> {
        self.provider
    }

    pub fn send_pull_request(&self) {
        // to provider
        // TODO: Implement
    }

    pub fn create_loop_message(&self) {
        // periodically
        // TODO: Implement loop message creation
    }

    pub fn create_drop_message(&self) {
        // periodically
        // TODO: Implement drop message creation
    }

    pub fn create_payload_message(&self, destination: NodeID) {
        // periodically
        // TODO: Implement payload message creation
    }
}

impl NodeBehavior for Client {
    fn process_loopix_message(&self, message: Message) {
        // do nothing basically
        // TODO: Implement
    }
}
