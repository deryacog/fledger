use super::core::{LoopixCore, LoopixConfig, LoopixStorage, NodeBehavior};
use super::messages::LoopixMessage;
use super::mixnode::MixnodeInterface;
use super::sphinx::Sphinx;
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use sphinx_packet::header::delays::Delay;
use sphinx_packet::SphinxPacket;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::thread;
use tokio::time::sleep;

#[derive(Debug)]
pub struct Provider {
    pub core: Arc<LoopixCore>,
    clients: RwLock<HashSet<NodeID>>,
    client_messages: Arc<RwLock<HashMap<NodeID, Vec<Sphinx>>>>,
}

impl Clone for Provider {
    fn clone(&self) -> Self {
        Provider {
            core: Arc::clone(&self.core),
            clients: RwLock::new(self.clients.read().unwrap().clone()),
            client_messages: Arc::new(RwLock::new(self.client_messages.read().unwrap().clone())),
        }
    }
}

impl PartialEq for Provider {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.core, &other.core)
            && *self.clients.read().unwrap() == *other.clients.read().unwrap()
            && *self.client_messages.read().unwrap() == *other.client_messages.read().unwrap()
    }
}

pub trait ProviderInterface: MixnodeInterface {
    fn subscribe_client(&mut self, client_id: NodeID);
    fn store_client_message(client_messages: &Arc<RwLock<HashMap<NodeID, Vec<Sphinx>>>>, client_id: NodeID, message: Sphinx);
    fn get_client_messages(&self, client_id: NodeID) -> Vec<Sphinx>;

    fn create_dummy_message(&self) -> Sphinx;
    fn send_pull_reply(&self, client_id: NodeID, message: Sphinx);
}

// LoopixConfig::new(
//     2.0,
//     500.0,
//     2.0,
//     3,
//     0.001,
//     500.0,
// ),
impl MixnodeInterface for Provider {
    fn new(core: Arc<LoopixCore>) -> Self {
        Self {
            core,
            client_messages: Arc::new(RwLock::new(HashMap::new())),
            clients: RwLock::new(HashSet::new()),
        }
    }

    fn process_forward_hop(&self, next_packet: Box<SphinxPacket>, next_address: NodeID, delay: Delay) {
        // store the message if the next_address if your client
        if self.clients.read().unwrap().contains(&next_address) {
            let client_messages = Arc::clone(&self.client_messages);
            let delay_duration = delay.to_duration();

            tokio::spawn(async move {
                sleep(delay_duration).await;
                let message = Sphinx { inner: *next_packet };
                Provider::store_client_message(&client_messages, next_address, message);
            });
        // act as a mixnode if you not
        } else {
            super::mixnode::MixnodeInterface::process_forward_hop(self, next_packet, next_address, delay);
        }
    }
}

impl ProviderInterface for Provider {
    fn subscribe_client(&mut self, client_id: NodeID) {
        self.clients.write().unwrap().insert(client_id);
    }

    fn store_client_message(client_messages: &Arc<RwLock<HashMap<NodeID, Vec<Sphinx>>>>, client_id: NodeID, message: Sphinx) {
        let mut messages = client_messages.write().expect("Failed to acquire write lock");
        messages.entry(client_id)
            .or_insert_with(Vec::new)
            .push(message);
    }

    fn get_client_messages(&self, client_id: NodeID) -> Vec<Sphinx> {
        let messages = self.client_messages.read().expect("Failed to acquire read lock");
        messages.get(&client_id).cloned().unwrap_or_default()
    }

    fn create_dummy_message(&self) -> Sphinx {
        // Sphinx::default()
        todo!()
    }

    fn send_pull_reply(&self, client_id: NodeID, message: Sphinx) {
        println!("Sending pull reply to client {:?}: {:?}", client_id, message);
    }
}

impl NodeBehavior for Provider {
    fn process_packet(&self, sphinx_packet: Sphinx){
        // route or store
        // TODO: Implement
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flarch::nodeids::NodeID;

    #[test]
    fn test_subscribe_client() {
        let mut provider = Provider::new(100);
        let client_id = NodeID::rnd();
        provider.subscribe_client(client_id);
        assert!(provider.clients.read().unwrap().contains(&client_id));
    }

    #[test]
    fn test_get_nonexistent_client_messages() {
        let provider = Provider::new(100);
        let client_id = NodeID::rnd();
        let messages = provider.get_client_messages(client_id);
        assert!(messages.is_empty());
    }

    #[test]
    fn test_provider_equality() {
        let provider1 = Provider::new(100);
        let provider2 = provider1.clone();
        assert_eq!(provider1, provider2);

        let mut provider3 = Provider::new(100);
        provider3.subscribe_client(NodeID::rnd());
        assert_ne!(provider1, provider3);
    }
}

