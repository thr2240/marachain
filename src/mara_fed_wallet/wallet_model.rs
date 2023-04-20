use serde::{Serialize, Deserialize};
use super::wallet_utils::{WalletList, get_deposit_address};

#[derive(Serialize)]
pub struct WalletInstallModel {
    pub mnemonic: String,
    pub peer_pub: String,
    pub peer_private: String,
}

#[derive(Serialize)]
pub struct WalletCommonModel {
    pub pubkey: String,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct WalletListModel {
    pub data: Vec<WalletList>,
}


pub struct DepositAddressModel {
    address: String,
}
impl  DepositAddressModel{
    pub fn new() -> DepositAddressModel {
        return DepositAddressModel {
            address: get_deposit_address().unwrap()
        };
    }
    pub fn get_deposit_address(&self) -> String {
        return get_deposit_address().unwrap();
    }
}