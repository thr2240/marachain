use actix_web::{
    web::{Json},
    Responder, Result
};
use rusqlite::Connection;
use serde_json::from_str;
use crate::mara_fed_wallet::{wallet_utils::{get_pub_key_from_cli}, wallet_model::WalletCommonModel};

use super::{wallet_model::{WalletInstallModel, DepositAddressModel, WalletListModel}, wallet_utils::{create_mnemonic, get_wallet_list, create_peer}, wallet_dto::AccountNumberDTO};

pub fn install() -> Result<impl Responder> {
    let peer_info = create_peer();
    let obj =  WalletInstallModel { mnemonic: create_mnemonic(), peer_pub: peer_info.0, peer_private: peer_info.1}; 
    Ok(Json(obj))
}

pub fn deposit_address() -> Result<impl Responder> {
    let obj: DepositAddressModel =  DepositAddressModel::new();
    Ok(Json(obj.get_deposit_address()))
}

pub fn list_wallets() -> Result<impl Responder> {
    let obj =  WalletListModel { data: get_wallet_list().unwrap()}; 
    Ok(Json(obj))
}