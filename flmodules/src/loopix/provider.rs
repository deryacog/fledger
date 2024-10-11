use super::core::{LoopixCore, LoopixConfig, LoopixStorage, NodeBehavior};
use super::messages::LoopixMessage;
use super::mixnode::MixnodeInterface;
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use super::sphinx::Sphinx;

#[derive(Debug, Clone, PartialEq)]
pub struct Provider {
    pub core: Arc<LoopixCore>,
    client_messages: HashMap<NodeID, Vec<Sphinx>>, // TODO: Define Message type
}

pub trait ProviderInterface: MixnodeInterface {
    fn subscribe_client(&mut self, client_id: NodeID);

    fn store_client_message(&mut self, client_id: NodeID, message: Sphinx);

    fn get_client_messages(&self) -> HashMap<NodeID, Vec<Sphinx>>;
    fn create_dummy_message(&self) -> Sphinx;
    fn send_pull_reply(&self, client_id: NodeID, message: Sphinx);
}

impl Provider {
    fn subscribe_client(&mut self, client_id: NodeID) {
        // TODO: Implement client subscription logic
    }

    fn store_client_message(&mut self, client_id: NodeID, message: Sphinx) {
        // TODO: Implement storing client messages
    }

    fn send_pull_reply(&self, client_id: NodeID, message: Sphinx) {
        // get_client_messages and check if at min
        // TODO: Implement sending pull reply to client
    }

    fn get_client_messages(&self) -> HashMap<NodeID, Vec<Sphinx>> {
        // TODO: Implement retrieving client messages
        HashMap::new()
    }

    // Any additional provider-specific methods can be added here
    fn process_loopix_message(&self, message: Sphinx) {
        // TODO: Implement processing loopix message
        // if for client storage
        // if not for client, rela
        //     super::mixnode::MixnodeInterface::process_loopix_message(self, message);
        // }
    }
}

impl MixnodeInterface for Provider {
    fn new(max_queue_size: usize) -> Self {
        // TODO: Generate key pair
        Self {
            core: Arc::new(LoopixCore::new(
                LoopixStorage::default(),
                LoopixConfig::new(
                    2.0,
                    500.0,
                    2.0,
                    3,
                    0.001,
                    500.0,
                ),
                max_queue_size,
            )),
            client_messages: HashMap::new(),
        }
    }

    // Implement other MixnodeInterface methods if needed
}

impl ProviderInterface for Provider {
    fn subscribe_client(&mut self, client_id: NodeID) {
        self.client_messages.entry(client_id).or_insert(Vec::new());
    }

    fn store_client_message(&mut self, client_id: NodeID, message: Sphinx) {
        if let Some(messages) = self.client_messages.get_mut(&client_id) {
            messages.push(message);
        }
    }

    fn get_client_messages(&self) -> HashMap<NodeID, Vec<Sphinx>> {
        self.client_messages.clone()
    }

    fn create_dummy_message(&self) -> Sphinx {
        // Dummy implementation, replace with actual dummy message creation logic
        // Sphinx::default()
        todo!()
    }

    fn send_pull_reply(&self, client_id: NodeID, message: Sphinx) {
        // Dummy implementation, replace with actual send logic
        println!("Sending pull reply to client {:?}: {:?}", client_id, message);
    }
}

impl NodeBehavior for Provider {
    fn process_packet(&self, sphinx_packet: Sphinx){
        // route or store
        // TODO: Implement
    }
}
