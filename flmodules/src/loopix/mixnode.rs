use std::sync::Arc;

use super::{core::LoopixCore, sphinx::Sphinx};
use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use super::messages::{LoopixMessage, LoopixOut};
use sphinx_packet::{
    header::delays::Delay,
    packet::*,
    payload::*,
    route::*,
};


use crate::{network::messages::NetworkIn, overlay::messages::NetworkWrapper};


#[derive(Debug, Clone, PartialEq)]
pub struct Mixnode {
    pub core: Arc<LoopixCore>,
}

pub trait MixnodeInterface {
    fn new(core: Arc<LoopixCore>) -> Self;

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
        let message =  LoopixOut::SphinxToNetwork(next_address, Sphinx { inner: *next_packet });
        self.core.send_message(delay.to_duration(), message);
    }

    fn create_loop_message(&self, node_id: NodeID) {
        // Default implementation
    }

    fn create_drop_message(&self, node_id: NodeID) {
        // Default implementation
    }

}
