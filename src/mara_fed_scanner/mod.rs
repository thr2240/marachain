use actix_web::{
    get,web,
    Responder, Result
 };
pub mod scanner_utils;
pub mod scanner_model;
pub mod scanner_service;
use crate::{ mara_fed_scanner::scanner_model::QueryOptions };
 /**
  * private function need to hide from external user on deployment
  */
  #[get("/listdeposits")]
  pub async fn listdeposits(opts: web::Query<QueryOptions>) -> Result<impl Responder> {
   let limit = 10;
   let offset = (opts.page.unwrap_or(1) - 1) * limit;
   let search_term = opts.search_term.to_string();
   return scanner_service::list_deposits(search_term,limit.try_into().unwrap(),offset.try_into().unwrap())
  }
  #[get("/listwithdraws")]
  pub async fn listwithdraws(opts: web::Query<QueryOptions>) -> Result<impl Responder> {
   let limit = 10;
   let offset = (opts.page.unwrap_or(1) - 1) * limit;
   let search_term = opts.search_term.to_string();
   return scanner_service::list_withdraws(search_term,limit.try_into().unwrap(),offset.try_into().unwrap())
  }