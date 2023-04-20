use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerMessageModel {
    pub peer_id: String,
    pub message_type: String,
    pub message: String
}
