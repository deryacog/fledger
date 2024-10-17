use std::sync::Arc;
use std::collections::HashMap;
use crate::overlay::messages::NetworkWrapper;

use super::{core::{LoopixConfig, LoopixCore, LoopixStorage}, sphinx::Sphinx};
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use sphinx_packet::route::Node;
use super::messages::LoopixMessage;


#[derive(Debug, Clone, PartialEq)]
pub struct Client {
    pub core: Arc<LoopixCore>,
    provider: Option<NodeID>,
    client_to_provider_map: HashMap<NodeID, NodeID>,
}

pub trait ClientInterface {
    fn new(max_queue_size: usize) -> Self;

    fn register_provider(&mut self, provider: NodeID);
    fn get_provider(&self) -> Option<NodeID>;
    fn send_pull_request(&self);

    fn create_loop_message(&self);
    fn create_drop_message(&self);
    fn create_payload_message(&self, destination: NodeID, msg: NetworkWrapper) -> (Node, Sphinx);

    
    // TODO some kind of send queue
}

// LoopixConfig::new(2.0, 500.0, 2.0, 3, 0.001, 0.0),
impl Client {
    pub fn new(core: Arc<LoopixCore>) -> Self {
        Self {
            core,
            provider: None,
            client_to_provider_map: HashMap::new(),
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

    pub fn create_payload_message(&self, destination: NodeID, msg: NetworkWrapper) -> (NodeID, Sphinx) {
        let dst_provider = self.get_provider_for_client(&destination);
        let route = self.core.create_route(self.get_provider(), dst_provider);

        let (_, sphinx) = self.core.create_sphinx_packet(destination, msg, &route);

        (self.get_provider().unwrap(), sphinx)
    }

    pub fn update_client_provider_mapping(&mut self, client_id: NodeID, new_provider_id: NodeID) {
        self.client_to_provider_map.insert(client_id, new_provider_id);
    }

    pub fn get_provider_for_client(&self, client_id: &NodeID) -> Option<NodeID> {
        self.client_to_provider_map.get(client_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn dummy_test() {
        assert_eq!(2, 2);
    }
}
