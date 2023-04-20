use actix_web::{
    web::{Json},
    Responder, Result
};
use rusqlite::Connection;

use super::{member_model::MemberListModel, member_utils::get_members};

pub fn listmembers() -> Result<impl Responder> {
    let conn = Connection::open(env!("DATABASE").to_owned()).unwrap(); 
    let obj =  MemberListModel { data: get_members(&conn, false).unwrap()}; 
    Ok(Json(obj))
}