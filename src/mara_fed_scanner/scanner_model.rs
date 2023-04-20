use serde::{Serialize, Deserialize};
use std::fmt::Display;
use std::fmt;

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletAddress {
    pub address: String
} 

#[derive(Serialize, Deserialize, Debug)]
pub struct PegModel {
    pub id: u32,
    pub receiver_address: String,
    pub tx_id: String,
    pub amount: f64,
    pub confirmations: u32,
    pub object_id: u32,
    pub object_type: u32,
} 

#[derive(Serialize, Deserialize, Debug)]
pub struct DepositModel {
    pub id: u32,
    pub receiver_address: String,
    pub tx_id: String,
    pub amount: f64,
    pub confirmations: u32,
} 



#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionResponse {
    pub result: TransactionModel
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionModel {
    pub confirmations: Option<u32>,
    pub txid: String,
    pub vout: Vec<serde_json::Value>,
    pub vin: Vec<serde_json::Value>
}

#[derive(Debug)]
pub enum StatusType {
    Start,
    Inprogress,
    Completed
}
impl Display for StatusType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug)]
pub enum TransactionType {
    DEPOSIT,
    WITHDRAW,
    MAINTRANSFER,
    PEGIN,
    PEGOUT,
}
impl Display for TransactionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VinModel {
    pub vin_txid: String,
}

#[derive(Serialize)]
pub struct DepositListModel {
    pub status: bool,
    pub message: String,
    pub totalDocs: u32,
    pub result: Vec<DepositList>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct DepositList {
    pub id: u32,
    pub sender_address: String,
    pub receiver_address: String,
    pub txid: String,
    pub amount: f64,
    pub confirmations: u32,
    pub status: String,
    pub date: String
}

#[derive(Debug, Deserialize)]
pub struct QueryOptions {
    pub page: Option<usize>,
    pub search_term: String 
}

#[derive(Serialize)]
pub struct TotalCount {
    pub cnt: u32
}