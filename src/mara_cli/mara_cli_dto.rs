use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CLIInputsDTO {
    pub txid: String,
    pub index: u32,
    pub amount: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CLIOutputsDTO {
    pub address: String,
    pub amount: f64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyRequestDTO {
    pub index: String,
    pub mnemonic: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PubRequestDTO {
    pub mnemonic: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DepositRequestDTO {
    pub addresses: Vec<String>,
    pub network: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransactionRequestDTO {
    pub inputs: Vec<CLIInputsDTO>,
    pub outputs: Vec<CLIOutputsDTO>,
    pub pubkeys: Vec<String>,
    pub redeem: Vec<String>,
    pub network: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SignatureRequestDTO {
    pub index: String,
    pub mnemonic: String,
    pub hex: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FinalizeRequestDTO {
    pub hex: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RedeemRequestDTO {
    pub pubkeys: Vec<String>,
}