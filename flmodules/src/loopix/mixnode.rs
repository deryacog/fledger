use std::sync::Arc;

use super::{core::LoopixCore, messages::{MessageType, MODULE_NAME}, sphinx::Sphinx};
use flarch::nodeids::NodeID;
use rand::seq::SliceRandom;
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
    fn get_core(&self) -> &Arc<LoopixCore>;

    fn create_loop_message(&self, node_id: NodeID) {
        // Default implementation
    }

    fn process_forward_hop(&self, next_packet: Box<SphinxPacket>, next_address: NodeID, delay: Delay);

    fn create_drop_message(&self) -> (NodeID, Sphinx) {
        // pick random provider
        let random_provider = self.get_core().providers.choose(&mut rand::thread_rng()).unwrap();

        // create route
        let route = self.get_core().create_route(None, Some(*random_provider));

        // create the networkmessage
        let drop_msg = serde_json::to_string(&MessageType::Drop).unwrap();
        let msg = NetworkWrapper{ module: MODULE_NAME.into(), msg: drop_msg};

        // create sphinx packet
        let (next_node, sphinx) = self.get_core().create_sphinx_packet(*random_provider, msg, &route);
        (LoopixCore::node_id_from_node_address(next_node.address), sphinx)
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

    fn get_core(&self) -> &Arc<LoopixCore> {
        &self.core
    }

    fn process_forward_hop(&self, next_packet: Box<SphinxPacket>, next_address: NodeID, delay: Delay) {
        let message =  LoopixOut::SphinxToNetwork(next_address, Sphinx { inner: *next_packet });
        self.core.send_message(delay.to_duration(), message);
    }

}
