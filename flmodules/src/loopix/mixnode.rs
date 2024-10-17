use std::sync::Arc;

use super::{core::{LoopixConfig, LoopixCore, LoopixStorage, NodeBehavior}, sphinx::Sphinx};
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use super::messages::LoopixMessage;
use sphinx_packet::{
    header::delays::Delay,
    packet::*,
    payload::*,
    route::*,
};

use super::super::ModuleMessage;

use crate::network::messages::NetworkIn;


#[derive(Debug, Clone, PartialEq)]
pub struct Mixnode {
    pub core: Arc<LoopixCore>,
}

pub trait MixnodeInterface {
    fn new(max_queue_size: usize) -> Self;

    fn create_loop_message(&self, node_id: NodeID) {
        // Default implementation
    }

    fn create_drop_message(&self, node_id: NodeID) {
        // Default implementation
    }

    fn process_forward_hop(&self, next_packet: Box<SphinxPacket>, next_address: NodeID, delay: Delay) {
    }

    // TODO some kind of send queue
}

// LoopixConfig::new(
//     2.0,
//     0.0,
//     0.0,
//     0,
//     0.001,
//     500.0,
// ),
impl MixnodeInterface for Mixnode {
    fn new(core: Arc<LoopixCore>) -> Self {
        Self { core }
    }

    fn process_forward_hop(&self, next_packet: Box<SphinxPacket>, next_address: NodeID, delay: Delay) {
        // Schedule the packet to be sent after the delay
        // TODO need to check how they do the queue
        let core:Arc<LoopixCore> = Arc::clone(&self.core);

        tokio::spawn(async move {
            tokio::time::sleep(delay.to_duration()).await;
            // Prepare packet for network module
            let module_message = ModuleMessage {
                module: "loopix".to_string(),
                msg: serde_json::to_string(&Sphinx { inner: *next_packet }).unwrap(),
            };
            // Return the message to be sent to the network module
            core.enqueue_packet(NetworkIn::SendNodeModuleMessage(next_address, module_message)).unwrap();
        });
    }

    fn create_loop_message(&self, node_id: NodeID) {
        // Default implementation
    }

    fn create_drop_message(&self, node_id: NodeID) {
        // Default implementation
    }

}

impl NodeBehavior for Mixnode {
    fn process_packet(&self, sphinx_packet: Sphinx){
        // basically routing
        // TODO: Implement
    }
}
