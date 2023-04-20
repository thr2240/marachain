use actix_web::{
    get,
    Responder, Result
 };
 pub mod member_utils;
 pub mod member_service;
 pub mod member_model;
 
 /**
  * private function need to hide from external user on deployment
  */
 #[get("/listmembers")]
 pub async fn listmembers() -> Result<impl Responder> {
    return member_service::listmembers()
 }