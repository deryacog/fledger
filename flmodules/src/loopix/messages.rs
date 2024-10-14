use std::error::Error;
use std::sync::Arc;

use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use sphinx_packet::{
    header::delays::Delay,
    packet::*,
    payload::*,
    route::*,
};
use tokio::time::sleep;

use crate::network::messages::*;
use super::super::ModuleMessage;
use super::mixnode;
use super::{
    client::Client, core::*, mixnode::{Mixnode, MixnodeInterface}, provider::{Provider, ProviderInterface}, sphinx::*
};
pub const MODULE_NAME: &str = "Loopix";

#[derive(Debug, Clone)]
pub enum NodeType {
    Client(Client),
    Mixnode(Mixnode),
    Provider(Provider),
}

impl NodeType {
    pub fn core(&self) -> &Arc<LoopixCore> {
        match self {
            NodeType::Client(client) => &client.core,
            NodeType::Mixnode(mixnode) => &mixnode.core,
            NodeType::Provider(provider) => &provider.core,
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
                // Extract the original message and forward it to the appropriate module
                if let Ok(module_message) = serde_json::from_str(std::str::from_utf8(&payload.recover_plaintext().unwrap()).unwrap()) {
                    vec![LoopixOut::ForwardToModule(module_message)]
                } else {
                    log::error!("Failed to deserialize payload");
                    vec![]
                }
            },
        }
    }

    /// Takes a msg from another module, wraps it in a sphinx packet and forwards it to the network module
    pub fn process_other_module_message(&mut self, node_id: Option<NodeID>, module_msg: ModuleMessage) -> Vec<LoopixOut> {
        match self {
            NodeType::Mixnode(_) | NodeType::Provider(_) => vec![],
            NodeType::Client(client) => {
                let dst = node_id.unwrap_or_else(|| NodeID::rnd());

                let sphinx = client.core.create_sphinx_packet(dst, module_msg);

                let loopix_msg = ModuleMessage {
                    module: MODULE_NAME.into(),
                    msg: serde_json::to_string(&sphinx).unwrap(),
                };

                let network_msg = ModuleMessage {
                    module: "Network".to_string(), // TODO change this to a variable
                    msg: serde_json::to_string(&loopix_msg).unwrap(),
                };

                let next_node = NodeID::rnd(); // TODO get this from sphinx header? 

                vec![LoopixOut::NodeModuleMessage(next_node, network_msg)]
            }
        }
    }

}

#[derive(Clone, Debug)]
pub enum LoopixMessage {
    Input(LoopixIn),
    Output(LoopixOut),
}

// The messages Loopix module might receive
// There are two options:
// 1. To loopix for processing: this type of message is received as ModuleMessage{ module: "Loopix", message: Sphinx packet } from Network module
//    (eventually from other node's Loopix module)
// 2. ModuleMessage from other modules (e.g., webproxy) that needs to be wrapped in a sphinx packet and sent to the network module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopixIn {
    // The Loopix module will know what to do with the message based on the 'module' field of ModuleMessage
    NodeModuleMessage(NodeID, ModuleMessage),
    ModuleMessage(ModuleMessage),
}

#[derive(Debug, Clone)]
pub enum LoopixOut {
    NodeModuleMessage(NodeID, ModuleMessage),
    ForwardToModule(ModuleMessage),
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

    fn process_message(&mut self, msg: LoopixIn) -> Vec<LoopixOut> { // TODO maybe get rid of this match case?
        match msg {
            LoopixIn::ModuleMessage(module_msg) => self.process_module_message(None, module_msg),
            LoopixIn::NodeModuleMessage(node_id, module_msg) => self.process_module_message(Some(node_id), module_msg),
        }
    }

    fn process_module_message(&mut self, node_id: Option<NodeID>, module_msg: ModuleMessage) -> Vec<LoopixOut> {
        if module_msg.module == MODULE_NAME {
            // This is a Loopix message which should be unwrapped
            if let Ok(sphinx) = serde_json::from_str::<Sphinx>(&module_msg.msg) {
                self.process_sphinx_packet(sphinx);
                vec![]
            } else {
                log::error!("Failed to deserialize Sphinx packet");
                vec![]
            }
        } else {
            self.role.process_other_module_message(node_id, module_msg)
        }
    }

    fn process_sphinx_packet(&mut self, sphinx_packet: Sphinx) {
        let processed = sphinx_packet.inner.process(self.role.core().get_secret_key()).unwrap();
        match processed {
            ProcessedPacket::ForwardHop(next_packet, next_address, delay) => {
                let next_node_id = LoopixCore::node_id_from_node_address(next_address);
                self.role.process_forward_hop(next_packet, next_node_id, delay);
            }
            ProcessedPacket::FinalHop(destination, surb_id, payload) => {
                // Check if the final destination matches our ID
                let dest = LoopixCore::node_id_from_destination_address(destination);
                if dest == self.our_id { // TODO meybe this is not the best idea for provider?
                    self.role.process_final_hop(dest, surb_id, payload);
                } else {
                    log::warn!("Received a FinalHop packet not intended for this node");
                }
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
