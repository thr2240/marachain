use actix_web::{
   get, post,
   web::{Bytes},
   Responder, Result
};
pub mod wallet_utils;
pub mod wallet_service;
pub mod wallet_model;
pub mod wallet_dto;
/**
 * private function need to hide from external user on deployment
 */
#[post("/install")]
pub async fn install() -> Result<impl Responder> {
   return wallet_service::install()
}


#[get("/depositaddress")]
pub async fn deposit_address() -> Result<impl Responder> {
   return wallet_service::deposit_address()
}

#[get("/listwallets")]
pub async fn list_wallets() -> Result<impl Responder> {
   return wallet_service::list_wallets()
}