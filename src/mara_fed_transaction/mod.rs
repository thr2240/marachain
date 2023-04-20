use actix_web::{
    get, post,
    web::{Bytes},
    Responder, Result,
    HttpRequest
 };
use actix_web::web::Query;
use qstring::QString;
use serde::{Deserialize, Serialize};

pub mod transaction_utils;
pub mod transaction_model;
pub mod transaction_dto;
pub mod transaction_service;

#[derive(Debug, Deserialize)]
pub struct Params {
    page: i64,
}

#[post("/sign")]
pub async fn sign(bytes: Bytes) -> Result<impl Responder> {
   let body: transaction_dto::SignDTO = serde_json::from_slice(&bytes).unwrap();
   return transaction_service::sign(&body)
}


#[get("/sum_of_deposit")]
pub async fn sum_of_deposit(req: HttpRequest) -> Result<impl Responder> {
    let params = req.query_string();
    let qs = QString::from(params);
    let text = qs.get("text").unwrap();
    let text_input = text.parse::<String>().unwrap();
    return transaction_service::get_service_deposits(text_input);
}

#[get("/sum_of_withdraw")]
pub async fn sum_of_withdraw(req: HttpRequest) -> Result<impl Responder> {
    let params = req.query_string();
    let qs = QString::from(params);
    let text = qs.get("text").unwrap();
    let text_input = text.parse::<String>().unwrap();
    return transaction_service::get_service_withdraws(text_input);
}

#[get("/chart_of_deposit")]
pub async fn chart_of_deposit(req: HttpRequest) -> Result<impl Responder> {
    let params = req.query_string();
    let qs = QString::from(params);
    let start_date = qs.get("start_date").unwrap();
    let end_date = qs.get("end_date").unwrap();
    let start_input = start_date.parse::<String>().unwrap();
    let end_input = end_date.parse::<String>().unwrap();
    return transaction_service::get_service_chart_deposits(start_input,end_input);
}

#[get("/chart_of_withdraw")]
pub async fn chart_of_withdraw(req: HttpRequest) -> Result<impl Responder> {
    let params = req.query_string();
    let qs = QString::from(params);
    let start_date = qs.get("start_date").unwrap();
    let end_date = qs.get("end_date").unwrap();
    let start_input = start_date.parse::<String>().unwrap();
    let end_input = end_date.parse::<String>().unwrap();
    return transaction_service::get_service_chart_withdraws(start_input,end_input);
}

#[get("/address_listing")]
pub async fn address_listing() -> Result<impl Responder> {
    return transaction_service::get_address_listing();
}

#[get("/history_listing")]
pub async fn history_listing(req: HttpRequest) -> Result<impl Responder> {
    let params = req.query_string();
    let qs = QString::from(params);
    let size = qs.get("page").unwrap();
    let text = qs.get("text").unwrap();
    let search_type = qs.get("type").unwrap();
    let page_size = format!("{:.8}", size).parse::<i64>().unwrap();
    let text_input = text.parse::<String>().unwrap();
    let search_type_input = search_type.parse::<i64>().unwrap();
    return transaction_service::get_history_listings(page_size, text_input,search_type_input);
}

#[get("/fee_listing")]
pub async fn fee_listing(req: HttpRequest) -> Result<impl Responder> {
    let params = req.query_string();
    let qs = QString::from(params);
    let text = qs.get("text").unwrap();
    let text_input = text.parse::<String>().unwrap();
    return transaction_service::get_fee_listings(text_input);
}

#[get("/balance_listing")]
pub async fn balance_listing(req: HttpRequest) -> Result<impl Responder> {
    let params = req.query_string();
    let qs = QString::from(params);
    let text = qs.get("text").unwrap();
    let text_input = text.parse::<String>().unwrap();
    return transaction_service::get_balance_listings(text_input);
}
