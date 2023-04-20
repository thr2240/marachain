use actix_web::{
    web::{Json},
    Responder, Result
};
use super::{scanner_model::{DepositListModel},scanner_utils::{get_deposits_list, get_withdraws_list,get_deposits_count,get_withdraws_count }};

pub fn list_deposits(search_term: String,limit: usize,offset: usize) -> Result<impl Responder> {
    let result = get_deposits_list(search_term.clone(),limit,offset).unwrap();
    let total_docs = get_deposits_count(search_term).unwrap();
    if result.len() == 0 {      
        let obj =  DepositListModel { status: false, message: "Something went wrong".to_string(), totalDocs: 0,result: result }; 
        return Ok(Json(obj));
    }
    let obj =  DepositListModel { status: true, message: "Deposit list retreived successfully".to_string(),totalDocs: total_docs, result: result }; 
    Ok(Json(obj))
}

pub fn list_withdraws(search_term: String,limit: usize,offset: usize) -> Result<impl Responder> {
    let result = get_withdraws_list(search_term.clone(),limit,offset).unwrap();
    let total_docs = get_withdraws_count(search_term).unwrap();
    if result.len() == 0 {      
        let obj =  DepositListModel { status: false, message: "Something went wrong".to_string(), totalDocs: 0,result: result }; 
        return Ok(Json(obj));
    }
    let obj =  DepositListModel { status: true, message: "Deposit list retreived successfully".to_string(),totalDocs: total_docs, result: result }; 
    Ok(Json(obj))
}