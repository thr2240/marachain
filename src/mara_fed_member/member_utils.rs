
use std::fs::File;
use std::io::Read;
use serde::{Serialize, Deserialize};
use rusqlite::{Connection, Result};

use crate::{mara_fed_peer::peer_model::PeerMessageModel, mara_fed_wallet::wallet_utils::{get_option_value, set_option_value}};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FedMember {
    pub identity: String,
    pub host: String,
    pub port: String,
    pub peer_id: String,
    pub user_pub: String,
    pub master_pub: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeMember {
    pub member: FedMember,
    pub pub_key: String,
    pub is_own: bool
}

pub fn get_members(conn: &Connection, is_active_only: bool) -> Result<Vec<FedMember>> {
    let mut stmt = conn.prepare("SELECT identity, host, port, peerId, user_pub, master_pub, status FROM federation_member")?;
    let account_list_iter = stmt.query_map([], |row| {
        Ok(FedMember {
            identity: row.get(0)?,
            host: row.get(1)?,
            port: row.get(2)?,
            peer_id: row.get(3)?,
            user_pub: row.get(4)?,
            master_pub: row.get(5)?,
            status: row.get(6)?
        })
    })?;

    let mut members = Vec::new();

    for member in account_list_iter {
        let mut master_item = member.unwrap();
        master_item.user_pub = strip_trailing_newline(&master_item.user_pub).to_string();
        master_item.master_pub = strip_trailing_newline(&master_item.master_pub).to_string();
        if is_active_only == false {
            members.push(master_item)
        } else {
            if master_item.status == "active" {
                members.push(master_item)
            }
        }
       
    }
    Ok(members)
}


pub fn update_peer_status(peer_id: String, status: String, conn: &Connection) -> Result<()> {
    let has_member: bool = check_peer_available(peer_id.to_string(), conn).unwrap();
    if !has_member  {
        return Ok(());
    }
    conn.execute(
        "UPDATE federation_member SET status=?1 WHERE peerId=?2",
        (&status, &peer_id),
     )?;
    Ok(())
}

pub fn check_peer_available(peer_id:String, conn: &Connection) -> Result <bool> {
    let mut stmt = conn.prepare("SELECT identity, host, port, peerId, user_pub, master_pub, status FROM federation_member WHERE peerId = :peer_id")?;

    let account_list_iter = stmt.query_map(&[(":peer_id", &peer_id.to_string())], |row| {
        Ok(FedMember {
            identity: row.get(0)?,
            host: row.get(1)?,
            port: row.get(2)?,
            peer_id: row.get(3)?,
            user_pub: row.get(4)?,
            master_pub: row.get(5)?,
            status: row.get(6)?
        })
    })?;

    let mut members = Vec::new();
    for account in account_list_iter {
        members.push(account.unwrap());
    }
    return Ok(members.len() != 0);
}

pub fn process_peer_message(peer_id: String, message: String, conn: &Connection) -> Result<()> {
    let response = serde_json::from_str::<PeerMessageModel>(&message).unwrap();
    if response.message_type == "current_peer" {
        let peer_id = get_option_value(&conn,"admin_peer".to_string()).unwrap();
        if peer_id != "".to_string() {
              let _ = set_option_value(response.message.to_string(), "admin_peer".to_string(), conn);
        }
    }
    Ok(())
}


fn strip_trailing_newline(input: &str) -> &str {
    input
        .strip_suffix("\r\n")
        .or(input.strip_suffix("\n"))
        .unwrap_or(input)
}
