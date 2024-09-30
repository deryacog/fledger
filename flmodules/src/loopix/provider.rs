use super::core::{LoopixCore, LoopixConfig, LoopixStorage, NodeBehavior};
use super::mixnode::MixnodeInterface;
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Provider {
    pub core: LoopixCore,
    client_messages: HashMap<NodeID, Vec<Message>>, // TODO: Define Message type
}

pub trait ProviderInterface: MixnodeInterface {
    fn subscribe_client(&mut self, client_id: NodeID);

    fn storage_client_message(&mut self, client_id: NodeID, message: Message);

    fn get_client_messages(&self) -> HashMap<NodeID, Vec<Message>>;
    fn create_dummy_message(&self) -> Message;
    fn send_pull_reply(&self, client_id: NodeID, message: Message);
}

impl Provider {
    fn subscribe_client(&mut self, client_id: NodeID) {
        // TODO: Implement client subscription logic
    }

    fn storage_client_message(&mut self, client_id: NodeID, message: Message) {
        // TODO: Implement storing client messages
    }

    fn send_pull_reply(&self, client_id: NodeID, message: Message) {
        // get_client_messages and check if at min
        // TODO: Implement sending pull reply to client
    }

    fn get_client_messages(&self) -> HashMap<NodeID, Vec<Message>> {
        // TODO: Implement retrieving client messages
        HashMap::new()
    }

    // Any additional provider-specific methods can be added here
    fn process_loopix_message(&self, message: Message) {
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
}

impl NodeBehavior for Provider {
    fn process_loopix_message(&self, message: Message) {
        // route or store
        // TODO: Implement
    }
}
