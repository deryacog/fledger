use super::core::{LoopixCore, LoopixConfig, LoopixStorage, NodeBehavior};
use super::mixnode::MixnodeInterface;
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use super::messages::LoopixMessage;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Provider {
    pub core: LoopixCore,
    client_messages: HashMap<NodeID, Vec<LoopixMessage>>, // TODO: Define Message type
}

pub trait ProviderInterface: MixnodeInterface {
    fn subscribe_client(&mut self, client_id: NodeID);

    fn store_client_message(&mut self, client_id: NodeID, message: LoopixMessage);

    fn get_client_messages(&self) -> HashMap<NodeID, Vec<LoopixMessage>>;
    fn create_dummy_message(&self) -> LoopixMessage;
    fn send_pull_reply(&self, client_id: NodeID, message: LoopixMessage);
}

impl Provider {
    fn subscribe_client(&mut self, client_id: NodeID) {
        // TODO: Implement client subscription logic
    }

    fn store_client_message(&mut self, client_id: NodeID, message: LoopixMessage) {
        // TODO: Implement storing client messages
    }

    fn send_pull_reply(&self, client_id: NodeID, message: LoopixMessage) {
        // get_client_messages and check if at min
        // TODO: Implement sending pull reply to client
    }

    fn get_client_messages(&self) -> HashMap<NodeID, Vec<LoopixMessage>> {
        // TODO: Implement retrieving client messages
        HashMap::new()
    }

    // Any additional provider-specific methods can be added here
    fn process_loopix_message(&self, message: LoopixMessage) {
        // TODO: Implement processing loopix message
        // if for client storage
        // if not for client, rela
        //     super::mixnode::MixnodeInterface::process_loopix_message(self, message);
        // }
    }
}

impl MixnodeInterface for Provider {
    fn new() -> Self {
        // TODO: Generate key pair
        Self {
            core: LoopixCore::new(
                LoopixStorage::default(),
                LoopixConfig {
                    lambda_loop: 2.0,
                    lambda_drop: 500.0,
                    lambda_payload: 2.0,
                    path_length: 3,
                    mean_delay: 0.001,
                    lambda_loop_mix: 500.0,
                },
            ),
            client_messages: HashMap::new(),
        }
    }

    // Implement other MixnodeInterface methods if needed
}

impl ProviderInterface for Provider {
    fn subscribe_client(&mut self, client_id: NodeID) {
        self.client_messages.entry(client_id).or_insert(Vec::new());
    }

    fn store_client_message(&mut self, client_id: NodeID, message: LoopixMessage) {
        if let Some(messages) = self.client_messages.get_mut(&client_id) {
            messages.push(message);
        }
    }

    fn get_client_messages(&self) -> HashMap<NodeID, Vec<LoopixMessage>> {
        self.client_messages.clone()
    }

    fn create_dummy_message(&self) -> LoopixMessage {
        // Dummy implementation, replace with actual dummy message creation logic
        // LoopixMessage::default()
        todo!()
    }

    fn send_pull_reply(&self, client_id: NodeID, message: LoopixMessage) {
        // Dummy implementation, replace with actual send logic
        println!("Sending pull reply to client {:?}: {:?}", client_id, message);
    }
}

impl NodeBehavior for Provider {
    fn process_loopix_message(&self, message: LoopixMessage) {
        // route or store
        // TODO: Implement
    }
}
