use std::error::Error;

use flarch::nodeids::NodeID;
use serde::{Deserialize, Serialize};
use sphinx_packet::{
    header::delays::Delay,
    packet::*,
    payload::*,
    route::*,
};
use tokio::time::sleep;

use crate::{network::messages::*, random_connections::messages::ModuleMessage};
use super::{
    core::*,
    sphinx::*,
};

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
    ModuleMessage(ModuleMessage),
}

#[derive(Debug, Clone)]
pub enum LoopixOut {
    NodeModuleMessage(NodeID, ModuleMessage),
    ForwardToModule(ModuleMessage),
}

#[derive(Debug)]
pub struct LoopixMessages {
    pub core: LoopixCore,
    our_id: NodeID,
}

impl LoopixMessages {
    pub fn new(
        storage: LoopixStorage,
        cfg: LoopixConfig,
        our_id: NodeID,
    ) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            core: LoopixCore::new(storage, cfg),
            our_id,
        })
    }

    pub fn process_messages(&mut self, msgs: Vec<LoopixIn>) -> Vec<LoopixOut> {
        msgs.into_iter()
            .flat_map(|msg| self.process_message(msg))
            .collect()
    }

    fn process_message(&mut self, msg: LoopixIn) -> Vec<LoopixOut> {
        match msg {
            LoopixIn::ModuleMessage(module_msg) => {
                if module_msg.module == "loopix" {
                    // This is a Sphinx packet from another Loopix module
                    if let Ok(sphinx) = serde_json::from_str::<Sphinx>(&module_msg.msg) {
                        self.process_sphinx_packet(sphinx)
                    } else {
                        log::error!("Failed to deserialize Sphinx packet");
                        vec![]
                    }
                } else {
                    self.process_other_module_message(module_msg)
                }
            }
        }
    }

    fn process_sphinx_packet(&mut self, sphinx_packet: Sphinx) -> Vec<LoopixOut> {
        match self.core.process_packet(sphinx_packet) {
            ProcessedPacket::ForwardHop(next_packet, next_address, delay) => {
                // Schedule the packet to be sent after the delay
                // TODO need to check how they do the queue
                tokio::spawn(async move {
                    tokio::time::sleep(delay.to_duration()).await;
                    // Prepare packet for network module
                    let next_node_id = NodeID::from(next_address.as_bytes());
                    let module_message = ModuleMessage {
                        module: "loopix".to_string(),
                        msg: serde_json::to_string(&Sphinx { inner: *next_packet }).unwrap(),
                    };
                    // Return the message to be sent to the network module
                    vec![LoopixOut::NodeModuleMessage(next_node_id, module_message)]
                });
                vec![]
            }
            ProcessedPacket::FinalHop(destination, surb_id, payload) => {
                // Check if the final destination matches our ID
                let dest = NodeID::from(destination.as_bytes());
                if dest == self.our_id {
                    // Extract the original message and forward it to the appropriate module
                    if let Ok(module_message) = serde_json::from_str(std::str::from_utf8(&payload.recover_plaintext().unwrap()).unwrap()) {
                        vec![LoopixOut::ForwardToModule(module_message)]
                    } else {
                        log::error!("Failed to deserialize payload");
                        vec![]
                    }
                } else {
                    log::warn!("Received a FinalHop packet not intended for this node");
                    vec![]
                }
            }
        }
    }

    fn process_other_module_message(&mut self, module_msg: ModuleMessage) -> Vec<LoopixOut> {
        // Create a new Sphinx packet
        let (packet, recipient) = self.core.create_sphinx_packet(module_msg);
        
        // Send the packet to the network module
        let loopix_message = ModuleMessage {
            module: "loopix".to_string(),
            msg: serde_json::to_string(&packet).unwrap(),
        };
        vec![LoopixOut::NodeModuleMessage(recipient, loopix_message)]
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
