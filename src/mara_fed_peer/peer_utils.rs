use actix_web::Result;
use peerlib::futures::channel::mpsc::{Sender, self};
use peerlib::libp2p::core::{ConnectedPoint, Endpoint};
use peerlib::libp2p::gossipsub::IdentTopic;
use rusqlite::Connection;
use std::error::Error;
use peerlib::futures::{prelude::*, select};
use peerlib::identity::{MyBehaviourEvent, MyBehaviour};
use peerlib::{identity::MaraPeer};
use peerlib::libp2p::{
    gossipsub,
    mdns,
    identity,
    PeerId,
    swarm::{SwarmEvent}
};
use walletlib::bitcoin::hashes::hex::{FromHex};
use serde::{Serialize, Deserialize};
use crate::mara_fed_member::member_utils::{check_peer_available, process_peer_message};
use crate::{mara_fed_member::member_utils::{update_peer_status}};

#[derive(Serialize, Deserialize, Debug)]
pub struct MyMessage {
    pub message: String,
}

#[derive(Clone)]
pub struct SwarmHandle {
    sender: Sender<(MyMessage, IdentTopic)>,
    topic: IdentTopic,
}

impl SwarmHandle {
    pub async fn new() -> Result<SwarmHandle, Box<dyn Error>> {
        let (sender, mut receiver) = mpsc::channel(100);
        let protobuf = Vec::<u8>::from_hex(&env!("PEER_PRIVATE").to_owned()).unwrap();
        let local_key = identity::Keypair::from_protobuf_encoding(&protobuf).unwrap();

        let mara_peer_result = MaraPeer::new(local_key).await.unwrap();
        let swarm_handle = SwarmHandle {sender: sender.clone(), topic: mara_peer_result.topic.clone() };
        let mut mara_peer_swarm = mara_peer_result.swarm;
        let conn = Connection::open(env!("DATABASE").to_owned()).unwrap(); 
        let mut current_domain = "".to_string();
        actix_rt::spawn(async move {
            loop {
                select! {
                    sibling = receiver.next() => {
                        let (sibling, topic) = sibling.unwrap();
                        if let Err(e) = mara_peer_swarm
                            .behaviour_mut().gossipsub
                            .publish(topic, (sibling.message.to_string()).as_bytes()) {
                        }
                    },
                    event = mara_peer_swarm.select_next_some() => match event {
                        SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer_id, _multiaddr) in list {
                                println!("mDNS discovered a new peer: {peer_id}");
                                let has_member: bool = check_peer_available(peer_id.to_string(), &conn).unwrap();
                                if has_member  {
                                    mara_peer_swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                                }
                       
                            }
                        },
                        SwarmEvent::Behaviour(MyBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                            for (peer_id, _multiaddr) in list {
                                println!("mDNS discover peer has expired: {peer_id}");
                                mara_peer_swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                            }
                        },
                        SwarmEvent::Behaviour(MyBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                            propagation_source: peer_id,
                            message_id: id,
                            message,
                        })) => {
                            process_peer_message(peer_id.to_string(), String::from_utf8_lossy(&message.data).to_string(), &conn);
                           }, 
                            SwarmEvent::NewListenAddr { address, .. } => {
                                println!("Listening on {:?}", address);
                                current_domain = address.to_string();
                            },
                            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                                println!("Established connection from peer");
                                let _ = update_peer_status(peer_id.to_string(),"active".to_string(),&conn);
                            },
                            SwarmEvent::ConnectionClosed { peer_id, .. } => {

                                println!("Connection closed from peer");
                                let _ = update_peer_status(peer_id.to_string(),"inactive".to_string(),&conn);
                            },
                        _ => {}
                    }
                }
            }
        });

        Ok(swarm_handle)
    }
    
    pub async fn publish(&mut self, message: String ) {
        self.sender.send((MyMessage{message}, self.topic.clone())).await.unwrap();
    }
}
