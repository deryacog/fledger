use std::sync::Arc;

use super::super::ModuleMessage;

use super::{core::{LoopixConfig, LoopixCore, LoopixStorage, NodeBehavior}, sphinx::Sphinx};
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use super::messages::LoopixMessage;


#[derive(Debug, Clone, PartialEq)]
pub struct Client {
    pub core: Arc<LoopixCore>,
    provider: Option<NodeID>,
    // mixnodes: Vec<NodeID>, // maybe?
}

pub trait ClientInterface {
    fn new(max_queue_size: usize) -> Self;

    fn register_provider(&mut self, provider: NodeID);
    fn get_provider(&self) -> Option<NodeID>;
    fn send_pull_request(&self);

    fn create_loop_message(&self);
    fn create_drop_message(&self);
    fn create_payload_message(&self, destination: NodeID);
    
    // TODO some kind of send queue
}

impl Client {
    pub fn new(max_queue_size: usize) -> Self {
        Self {
            core: Arc::new(LoopixCore::new(
                LoopixStorage::default(),
                LoopixConfig::new(2.0, 500.0, 2.0, 3, 0.001, 0.0),
                max_queue_size,
            )),
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
    fn process_packet(&self, sphinx_packet: Sphinx){
        // do nothing basically
        // TODO: Implement
    }
}
