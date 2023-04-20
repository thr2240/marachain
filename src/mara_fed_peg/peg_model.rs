use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DepositInfoModel {
    pub id: u32,
    pub sender_address: String,
    pub tx_id: String,
    pub amount: f64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SpenderTreePaths {
    pub tree: Vec<String>,
}

