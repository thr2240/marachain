use super::{transaction_dto::SignDTO, transaction_model::{ChartDepositModel, ChartWithdrawModel, SumDepositModel, SumWithdrawModel, ListHistoryModel, ListAddressModel, FeeDataModel, SignModel,BalanceDataModel}, transaction_utils::{chart_of_deposit, chart_of_withdraw, list_of_history, list_of_address, total_history_count, fee_listing,sign_from_cli, balance_listing}};
use actix_web::{
    web::{Json},
    Responder, Result
};
use rusqlite::Connection;

pub fn sign(params: &SignDTO) -> Result<impl Responder> {
    let conn = Connection::open(env!("DATABASE").to_owned()).unwrap();
    let obj =  SignModel { hex: sign_from_cli(params, &conn).unwrap()}; 
    Ok(Json(obj))
}

pub fn get_service_deposits(text: String) -> Result<impl Responder> {
    let obj: SumDepositModel = SumDepositModel::new(text.clone());
    Ok(Json(obj.sum_of_deposit(text.clone())))
}

pub fn get_service_withdraws(text: String) -> Result<impl Responder> {
    let obj: SumWithdrawModel = SumWithdrawModel::new(text.clone());
    Ok(Json(obj.sum_of_withdraw(text.clone())))
}

pub fn get_service_chart_deposits(start:String, end:String) -> Result<impl Responder> {
    let obj = ChartDepositModel {
        data: chart_of_deposit(start,end).unwrap(),
    };
    Ok(Json(obj))
}

pub fn get_service_chart_withdraws(start:String, end:String) -> Result<impl Responder> {
    let obj = ChartWithdrawModel {
        data: chart_of_withdraw(start,end).unwrap(),
    };
    Ok(Json(obj))
}


pub fn get_address_listing() -> Result<impl Responder> {
    let obj = ListAddressModel {
        data: list_of_address().unwrap(),
    };
    Ok(Json(obj))
}

pub fn get_history_listings(page_size: i64, text: String, types: i64) -> Result<impl Responder> {
    let obj = ListHistoryModel {
        data: list_of_history(page_size, text.clone(), types.clone()).unwrap(),
        total: total_history_count(text.clone(), types.clone()).unwrap(),
    };
    Ok(Json(obj))
}

pub fn get_fee_listings(text:String) -> Result<impl Responder> {
    let obj = FeeDataModel {
        data: fee_listing(text).unwrap(),
    };
    Ok(Json(obj))
}

pub fn get_balance_listings(text: String) -> Result<impl Responder> {
    let obj = BalanceDataModel {
        data: balance_listing(text).unwrap(),
    };
    Ok(Json(obj))
}
