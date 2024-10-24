use std::error::Error;
use std::sync::Arc;

use flarch::nodeids::{NodeID, NodeIDs};
use serde::{Deserialize, Serialize};
use sphinx_packet::{
    header::delays::Delay,
    packet::*,
    payload::*,
    route::*,
};
use tokio::time::sleep;
use serde_json;

use crate::{network::messages::*, nodeconfig::NodeInfo};
use crate::overlay::messages::NetworkWrapper;
use super::{
    client::Client, core::*, mixnode::{Mixnode, MixnodeInterface}, provider::{Provider, ProviderInterface}, sphinx::*
};
pub const MODULE_NAME: &str = "Loopix";


#[derive(Clone, Debug)]
pub enum LoopixMessage {
    Input(LoopixIn),
    Output(LoopixOut),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopixIn {
    // message from overlay: needs to be put in a sphinx packet
    OverlayRequest(NodeID, NetworkWrapper),
    // packet in sphinx format, from other nodes
    SphinxFromNetwork(Sphinx),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Payload(NetworkWrapper),
    Drop,
    Loop,
    Dummy,
    PullRequest,
    SubscriptionRequest, // TODO    
    SubscriptionReply, // TODO
}


#[derive(Debug, Clone)]
pub enum LoopixOut {
    // Sphinx packet Loopix: network will forward it to the next node
    SphinxToNetwork(NodeID, Sphinx),
    // Unencrypted module message to overlay
    OverlayReply(NodeID, NetworkWrapper),
    //
    NodeInfosConnected(Vec<NodeInfo>),

    NodeIDsConnected(NodeIDs),

    NodeInfoAvailable(Vec<NodeInfo>),
}

#[derive(Debug)]
pub struct LoopixMessages {
    pub role: NodeType,
    our_id: NodeID,
}

impl LoopixMessages {
    pub fn new(
        our_id: NodeID,
        node_type: NodeType,
    ) -> Self {
        Self {
            role: node_type,
            our_id,
        }
    }

    pub fn process_messages(&mut self, msgs: Vec<LoopixIn>) -> Vec<LoopixOut> {
        msgs.into_iter()
            .flat_map(|msg| self.process_message(msg))
            .collect()
    }

    fn process_message(&mut self, msg: LoopixIn) -> Vec<LoopixOut> {
        match msg {
            LoopixIn::OverlayRequest(node_id, message) => self.role.process_overlay_message(node_id, message),
            LoopixIn::SphinxFromNetwork(sphinx) => self.process_sphinx_packet(sphinx),
        }
    }

    fn process_sphinx_packet(&mut self, sphinx_packet: Sphinx) -> Vec<LoopixOut> {
        let processed = sphinx_packet.inner.process(self.role.core().get_secret_key()).unwrap();
        match processed {
            ProcessedPacket::ForwardHop(next_packet, next_address, delay) => {
                let next_node_id = LoopixCore::node_id_from_node_address(next_address);
                self.role.process_forward_hop(next_packet, next_node_id, delay);
                vec![]
            }
            ProcessedPacket::FinalHop(destination, surb_id, payload) => {
                // Check if the final destination matches our ID
                let dest = LoopixCore::node_id_from_destination_address(destination);
                if dest == self.our_id {
                    self.role.process_final_hop(dest, surb_id, payload);
                } else {
                    log::warn!("Received a FinalHop packet not intended for this node");
                }
                vec![]
            }
        }
    }
}


#[derive(Debug, Clone)]
pub enum NodeType {
    Client(Client),
    Mixnode(Mixnode),
    Provider(Provider),
}

impl NodeType {
    pub fn core(&self) -> Arc<LoopixCore> {
        match self {
            NodeType::Client(client) => Arc::clone(&client.core),
            NodeType::Mixnode(mixnode) => Arc::clone(&mixnode.core),
            NodeType::Provider(provider) => Arc::clone(&provider.core),
        }
    }

    pub fn process_forward_hop(&self, next_packet: Box<SphinxPacket>, next_address: NodeID, delay: Delay){
        match self {
            NodeType::Client(_) => {},
            NodeType::Mixnode(mixnode) => {
                mixnode.process_forward_hop(next_packet, next_address, delay);
            },
            NodeType::Provider(provider) => {
                // provider.store_client_message(next_address, payload);
                // TODO store client message and then or forward the message
                // TODO pull request
            }
        }
    }

    pub fn process_final_hop(&self, destination: NodeID, surb_id: [u8; 16], payload: Payload)-> Vec<LoopixOut>{
        match self {
            NodeType::Mixnode(_) | NodeType::Provider(_)=> vec![], // TODO I guess this is dropping for loop messages?
            NodeType::Client(_) => {
                // Extract the original message to be sent to overlay
                if let Ok(module_message) = serde_json::from_str::<NetworkWrapper>(std::str::from_utf8(&payload.recover_plaintext().unwrap()).unwrap()) {
                    vec![LoopixOut::OverlayReply(destination, module_message)]
                } else {
                    log::error!("Failed to deserialize payload");
                    vec![]
                }
            },
        }
    }

    /// Takes a msg from another module, wraps it in a sphinx packet
    pub fn process_overlay_message(&mut self, dst: NodeID, message: NetworkWrapper) -> Vec<LoopixOut> {
        match self {
            NodeType::Mixnode(_) | NodeType::Provider(_) => vec![],
            NodeType::Client(client) => {
                let (next_node, sphinx) = client.create_payload_message(dst, message);
                vec![LoopixOut::SphinxToNetwork(next_node, sphinx)]
            }
        }
    }

}

impl From<LoopixIn> for LoopixMessage {
    fn from(msg: LoopixIn) -> Self {
        LoopixMessage::Input(msg)
    }
}

impl From<LoopixOut> for LoopixMessage {
    fn from(msg: LoopixOut) -> Self {
        LoopixMessage::Output(msg)
    }
}
