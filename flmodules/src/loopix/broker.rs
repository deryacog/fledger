use tokio::sync::mpsc::{self, Receiver};
use std::time::Duration;
use flarch::nodeids::NodeID;

use flarch::{
    broker::{Broker, BrokerError, Subsystem, SubsystemHandler},
    platform_async_trait,
};

use crate::{
    overlay::messages::{OverlayIn, OverlayMessage, OverlayOut, NetworkWrapper},
    random_connections::messages::{RandomIn, RandomMessage, RandomOut},
    timer::TimerMessage,
    network::messages::{NetworkIn, NetworkMessage, NetworkOut},
};

use super::{
    core::{LoopixConfig, LoopixCore, LoopixStorage},
    messages::{NodeType, LoopixIn, LoopixMessage, LoopixMessages, LoopixOut},
    client::Client, mixnode::Mixnode, provider::Provider, sphinx::Sphinx
};

const MODULE_NAME: &str = "Loopix";

pub struct LoopixBroker {
    pub broker: Broker<LoopixMessage>,
}

impl LoopixBroker {
    pub async fn start(
        overlay: Broker<OverlayMessage>,
        mut network: Broker<NetworkMessage>,
        our_id: NodeID,
        mut receiver: Receiver<(Duration, LoopixOut)>,
        role: NodeType,
    ) -> Result<Broker<LoopixMessage>, BrokerError> {
        let mut broker = Broker::new();

        let core = role.core();

        broker.link_bi(
            overlay.clone(),
            Box::new(Self::from_overlay),
            Box::new(Self::to_overlay),
        ).await?;

        broker.link_bi(
            network.clone(),
            Box::new(Self::from_network),
            Box::new(Self::to_network),
        ).await?;

        broker.add_subsystem(Subsystem::Handler(Box::new(LoopixTranslate {
            overlay,
            network: network.clone(),
            loopix_messages: LoopixMessages::new(our_id, role),
        }))).await?;

        let lambda_payload = Duration::from_secs_f64(core.get_config().lambda_payload());
        Self::start_receiver_thread(receiver, network, lambda_payload);

        Ok(broker)
    }

    fn from_overlay(msg: OverlayMessage) -> Option<LoopixMessage> {
        if let OverlayMessage::Input(OverlayIn::NetworkWrapperToNetwork(node_id, wrapper)) = msg {
            return Some(LoopixIn::OverlayRequest(node_id, wrapper).into());
        }
        None
    }

    fn from_network(msg: NetworkMessage) -> Option<LoopixMessage> {
        if let NetworkMessage::Output(NetworkOut::RcvLoopixMessage(node_id, message)) = msg {
            let sphinx_packet: Sphinx = serde_json::from_str(&message).unwrap();
            return Some(LoopixIn::SphinxMessage(sphinx_packet).into());
        }
        None
    }

    fn to_overlay(msg: LoopixMessage) -> Option<OverlayMessage> {
        if let LoopixMessage::Output(LoopixOut::OverlayReply(destination, module_msg)) = msg {
            return Some(OverlayOut::NetworkMapperFromNetwork(destination, module_msg).into());
        }
        None
    }

    fn to_network(msg: LoopixMessage) -> Option<NetworkMessage> {
        if let LoopixMessage::Output(LoopixOut::SphinxToNetwork(node_id, sphinx)) = msg {
            let msg = serde_json::to_string(&sphinx).unwrap();
            return Some(NetworkIn::SendLoopixMessage(node_id, msg).into());
        }
        None
    }

    pub fn start_receiver_thread(mut receiver: Receiver<(Duration, LoopixOut)>, mut network: Broker<NetworkMessage>, payload_rate: Duration) {
        tokio::spawn(async move {
            let mut sphinx_messages: Vec<(Duration, LoopixOut)> = Vec::new();

            loop {
                // Wait for send delay
                tokio::time::sleep(payload_rate).await;

                // Subtract the wait duration from all message delays
                for (delay, _) in &mut sphinx_messages {
                    *delay = delay.saturating_sub(payload_rate);
                }

                // Receive new messages
                if let Some((delay, loopix_out)) = receiver.recv().await {
                    sphinx_messages.push((delay, loopix_out));
                }

                // Sort messages by remaining delay
                sphinx_messages.sort_by_key(|&(delay, _)| delay);

                // Emit messages with 0 or less delay
                if let Some((delay, loopix_out)) = sphinx_messages.first() {
                    if *delay <= Duration::ZERO {
                        if let LoopixOut::SphinxToNetwork(node_id, sphinx) = loopix_out {
                            let msg = serde_json::to_string(&sphinx).unwrap();
                            network.emit_msg(NetworkIn::SendLoopixMessage(*node_id, msg).into()).unwrap();
                            sphinx_messages.remove(0);
                        }
                    }
                }
            }
        });
    }
}

struct LoopixTranslate {
    overlay: Broker<OverlayMessage>,
    network: Broker<NetworkMessage>,
    loopix_messages: LoopixMessages,
}

#[platform_async_trait()]
impl SubsystemHandler<LoopixMessage> for LoopixTranslate {
    async fn messages(&mut self, msgs: Vec<LoopixMessage>) -> Vec<LoopixMessage> {
        let mut outgoing_msgs = vec![];

        for msg in msgs {
            if let LoopixMessage::Input(loopix_in) = msg {
                let processed_msgs = self.loopix_messages.process_messages(vec![loopix_in]);
                outgoing_msgs.extend(processed_msgs.into_iter().map(LoopixMessage::Output));
            }
        }

        // for msg in msgs {
        //     match msg {
        //         LoopixMessage::Input(loopix_in) => {
        //             let processed_msgs = self.loopix_messages.process_messages(vec![loopix_in]);
        //             outgoing_msgs.extend(processed_msgs.into_iter().map(LoopixMessage::Output));
        //         }
        //         LoopixMessage::Output(LoopixOut::OverlayReply(node_id, module_msg)) => {
        //             self.overlay.emit_msg(OverlayOut::NetworkMapperFromNetwork(node_id, module_msg).into()).unwrap();
        //         }
        //         LoopixMessage::Output(LoopixOut::SphinxToNetwork(node_id, sphinx)) => {
        //             let msg = serde_json::to_string(&sphinx).unwrap();
        //             self.network.emit_msg(NetworkIn::SendLoopixMessage(node_id, 
        //                 msg).into()).unwrap();
        //         }
        //     }
        // }

        outgoing_msgs
    }
}