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

use crate::network::messages::*;
use super::{
    core::*,
    sphinx::*,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Module {
    Network,
    WebProxy,
    Loopix,
}

// message to put into a sphinx payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMessage {
    module: Module,
    content: Vec<u8>
}

#[derive(Clone, Debug)]
pub enum LoopixMessage {
    Input(LoopixIn),
    Output(LoopixOut),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoopixIn {
    Sphinx(Sphinx),
}

#[derive(Debug, Clone)]
pub enum LoopixOut {
    ForwardToModule(ModuleMessage), // TODO I don't think this works
    ForwardToNetwork(NetworkIn)
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

    pub async fn process_messages(&mut self, msgs: Vec<LoopixIn>) -> Vec<LoopixOut> {
        let mut out = vec![];
        for msg in msgs {
            log::trace!("Got msg: {msg:?}");
            out.extend(match msg {
                LoopixIn::Sphinx(packet) => self.process_message(packet).await,
            });
        }
        out
    }

    async fn process_message(&mut self, packet: Sphinx) -> Vec<LoopixOut> {
        let processed = packet.inner.process(self.core.get_secret_key()).unwrap();
        match processed {
            ProcessedPacket::ForwardHop(next_packet, next_address, delay) => {
                self.process_forward_hop(next_packet, next_address, delay).await
            },
            ProcessedPacket::FinalHop(destination, surb_id, payload) => {
                self.process_final_hop(destination, surb_id, payload).await
            },
        }
    }

    async fn process_forward_hop(&mut self, next_packet: Box<SphinxPacket>, next_address: NodeAddressBytes, delay: Delay) -> Vec<LoopixOut> {
        sleep(delay.to_duration()).await;

        let next_addr = NodeID::from(next_address.as_bytes());

        let sphinx = Sphinx { inner: *next_packet };
        // TODO: add to Network a message that takes bytes
        
        let message_content = serde_json::to_string(&sphinx).unwrap();
        let network_message = NetworkIn::SendNodeMessage(next_addr, message_content);

        vec![LoopixOut::ForwardToNetwork(network_message)]
    }

    async fn process_final_hop(&mut self, destination: DestinationAddressBytes, surb_id: SURBIdentifier, payload: Payload) -> Vec<LoopixOut> {
        let dest_addr = NodeID::from(destination.as_bytes());
        
        if dest_addr != self.our_id {
            return vec![];
        }

        let vec_message = payload.recover_plaintext().unwrap();
        // TODO convert vec message into a module message

        // TODO: not sure what to with identifier
        todo!()
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
