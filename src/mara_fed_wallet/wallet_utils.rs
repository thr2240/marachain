use serde_json::from_str;
use walletlib::mnemonic::Mnemonic;
use walletlib::account::MasterKeyEntropy;
use serde::{Serialize, Deserialize};
use rusqlite::{Connection, Result};
use curl::easy::Easy;
use crate::mara_fed_member::member_utils::{get_members, update_peer_status};
use crate::mara_fed_peer::peer_model::PeerMessageModel;
use crate::mara_fed_peer::peer_utils::SwarmHandle;
use crate::mara_fed_transaction::transaction_utils::get_pool_info;
use crate::{mara_fed_member::member_utils::FedMember, mara_cli::{mara_cli_dto::{KeyRequestDTO, DepositRequestDTO}, mara_cli_service::execute_cli}};
use super::wallet_model::WalletListModel;
use peerlib::libp2p;
use walletlib::bitcoin::hashes::hex::{ToHex};

#[derive(Serialize, Deserialize, Debug)]
pub struct WalletList {
    pub mutlisig_address: String,
    pub indexer: u32
}

pub struct OptionList {
    value: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MultiSigResponse {
    result: MultiSigModel
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MultiSigModel {
    address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PubkeyResponse {
    pub pubkey: String
}

pub fn create_mnemonic() -> String {
    let mnemonic = Mnemonic::new_random(MasterKeyEntropy::Sufficient).unwrap();
    return mnemonic.to_string()
}

pub fn create_peer() -> (String, String) {
    let local_key = libp2p::identity::Keypair::generate_ed25519();
    let local_peer_id = libp2p::PeerId::from(local_key.public());
    return (local_peer_id.to_string(), local_key.to_protobuf_encoding().unwrap().to_hex())
}

pub fn get_wallet_list() -> Result <Vec<WalletList>> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut stmt = conn.prepare("SELECT mutlisig_address, indexer FROM federation_address")?;

    let account_list_iter = stmt.query_map([], |row| {
        Ok(WalletList {
            mutlisig_address: row.get(0)?,
            indexer: row.get(1)?
        })
    })?;

    let mut tokens = Vec::new();
    for account in account_list_iter {
        tokens.push(account.unwrap());
    }

    return Ok(tokens);
}

pub fn get_deposit_address() -> Result <String> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut stmt = conn.prepare("SELECT mutlisig_address, indexer FROM federation_address ORDER BY created_date DESC LIMIT 1 ")?;

    let account_list_iter = stmt.query_map([], |row| {
        Ok(WalletList {
            mutlisig_address: row.get(0)?,
            indexer: row.get(1)?
        })
    })?;

    let mut deposit_address = "".to_string();
    for account in account_list_iter {
        deposit_address = account.unwrap().mutlisig_address;
    }

    return Ok(deposit_address);
}

pub fn get_bitcoin_block_height_by_day(conn: &Connection) -> i64 {
    let mut bitcoin_block_height = from_str::<i64>(&get_option_value(&conn,"bitcoin_block_height".to_string()).unwrap()).unwrap();
    let remaining_count = bitcoin_block_height % 144;
    if remaining_count > 0 {
        bitcoin_block_height = bitcoin_block_height - remaining_count;
    }
    return bitcoin_block_height / 144
}

pub fn set_decision_maker(conn: &Connection) -> Result<()>{
   let members = get_members(conn, true).unwrap();
   let mut bitcoin_block_height = from_str::<usize>(&get_option_value(&conn,"bitcoin_block_height".to_string()).unwrap()).unwrap();

   let current_position = bitcoin_block_height %  members.len();
   let member_item = members[current_position].clone();
   let _ = set_option_value(member_item.peer_id.to_string(), "admin_peer".to_string(), conn);
   Ok(())
}

pub fn is_decision_maker(conn: &Connection) -> (bool, FedMember) {
    let mut position = 0;
    let members = get_members(conn, false).unwrap();
    let member_count = members.len();
    let mut current_member = FedMember { identity: "".to_string()
    , host: "".to_string(), port: "".to_string(), peer_id: "".to_string(), user_pub: "".to_string(), master_pub: "".to_string(), status: "".to_string() };
    for mut member  in members{
        if member.identity == env!("IDENTITY").to_owned() {
            current_member = member;
            break
        }
        position = position + 1;
    }
    let peer_id = get_option_value(&conn,"admin_peer".to_string()).unwrap();

    return (current_member.peer_id == peer_id, current_member)
}


pub fn node_scheduler(conn: &Connection) -> Result <()> {
    let fee_sidechain = get_pool_info("marachain".to_string(), &conn, "fee".to_string());
    if let Err(err) = fee_sidechain {
    }
    let fee_mainchain = get_pool_info("bitcoin".to_string(), &conn,"fee".to_string());
    if let Err(err) = fee_mainchain {
    }

    let height_sidechain = get_pool_info("marachain".to_string(), &conn, "blockheight".to_string());
    if let Err(err) = height_sidechain {
    }
    let height_mainchain = get_pool_info("bitcoin".to_string(), &conn,"blockheight".to_string());
    if let Err(err) = height_mainchain {
    }

    let account_number = get_bitcoin_block_height_by_day(conn) % 1000;
    let _ = create_new_account(account_number as u32, conn);

    Ok(())
}

pub fn create_new_account(account_number: u32,conn: &Connection)  -> Result <()>  {

    let members = get_members(&conn, false).unwrap();
    let has_account: bool = check_address_indexer(account_number as u32, &conn).unwrap();
    if has_account  {
        return Ok(());
    }

    let mut addresslist = Vec::new();
    for member in members {
        let account_pubkey = get_pub_key_from_cli(&account_number, member.user_pub.to_string()).unwrap();
        addresslist.push(account_pubkey);
    }

    let new_address = generate_deposit_address(addresslist).unwrap();
    if new_address != "".to_string() {
        let _ = save_address(new_address,account_number as u32, &conn);
    }

    Ok(())
}

pub fn generate_deposit_address(addresslist: Vec<String>) -> Result<String> {
    let request_obj = DepositRequestDTO {
        addresses : addresslist,
        network: env!("NETWORK").to_owned()
    };
    let params = serde_json::to_string(&request_obj).unwrap();

    let cli_response = execute_cli("-d".to_string(), params);
    if let Err(err) = cli_response {
        println!("error on deposit address gemerate cli {:?}",err);
        return Ok("".to_string());
    }
    let result = cli_response.unwrap();
    if result.status == false {
        return Ok("".to_string());
    };
    return Ok(result.result);
}

fn save_address(newaddress: String, indexer: u32,conn: &Connection) -> Result <()> {
    conn.execute(
        "INSERT INTO federation_address (indexer, mutlisig_address) VALUES (?1, ?2)",
        (&indexer.to_string(), &newaddress),
    )?;

    conn.execute("DELETE
    from federation_address
    WHERE id not in (
        SELECT id
        FROM federation_address
        ORDER BY created_date DESC
        LIMIT 10
    )", ())?;
    Ok(())
}

/**
 * sub node functions
 */
 pub  fn get_pub_key_from_cli(account_number: &u32, xpub_key: String) -> Result <String> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;

    let request_obj = KeyRequestDTO {
        index : account_number.to_string(),
        mnemonic: xpub_key
    };
    let params = serde_json::to_string(&request_obj).unwrap();

    let cli_response = execute_cli("-p".to_string(), params);
    if let Err(err) = cli_response {
        println!("error on key generate cli {:?}",err);
        return Ok("".to_string());
    }

    Ok(cli_response.unwrap().result)
}

fn check_address_indexer(account_number:u32, conn: &Connection) -> Result <bool> {
    let mut stmt = conn.prepare("SELECT mutlisig_address, indexer FROM federation_address WHERE indexer = :indexer")?;

    let account_list_iter = stmt.query_map(&[(":indexer", &account_number.to_string())], |row| {
        Ok(WalletList {
            mutlisig_address: row.get(0)?,
            indexer: row.get(1)?
        })
    })?;

    let mut addresses = Vec::new();
    for account in account_list_iter {
        addresses.push(account.unwrap());
    }
    return Ok(addresses.len() != 0);
}

fn get_wallet_list_from_management(manager:&FedMember) -> Result <WalletListModel> {
    let mut output = String::new();
    let urlstr = "http://".to_string()+ &manager.host + &":".to_string() + &manager.port + "/wallet/listwallets";
    let mut easy = Easy::new();
    easy.url(&urlstr).unwrap();
     { // Use the Rust-specific `transfer` method to allow the write function to borrow `output` temporarily
         let mut transfer = easy.transfer();
 
         transfer.write_function(|data| {
             output.push_str(&String::from_utf8_lossy(data));
             Ok(data.len())
         }).unwrap();
 
         // Actually execute the request
         let api_respose = transfer.perform();
         if let Err(err) = api_respose {
             println!("Management node connetion failure {:?}",err);
             let addresses = Vec::new();
             return Ok(WalletListModel { data: addresses });
         } else {
             api_respose.unwrap();
         }
    }
    println!("{:?}",output);
    let object: WalletListModel = serde_json::from_str::<WalletListModel>(&output).unwrap();
    Ok(object)
 }
 
 pub async fn sync_database() -> Result <()> {
    let conn = Connection::open(env!("DATABASE").to_owned()).unwrap(); 
    let _ = sync_address(&conn).await;
    let _ = reset_peer_status(&conn).await;
    let _ = clear_peg_queue(&conn).await;
    let _ = reset_admin_peer(&conn).await;
    Ok(())
}

pub async fn send_admin_peer(mut swarm_obj: SwarmHandle, conn: &Connection) {
    let peer_id = get_option_value(&conn,"admin_peer".to_string()).unwrap();
    if peer_id != "".to_string() {
        let msg_obj = PeerMessageModel {
            peer_id: env!("PEER_PUB").to_owned(),
            message_type: "current_peer".to_string(),
            message: peer_id
        };
        let params = serde_json::to_string(&msg_obj).unwrap();
        swarm_obj.publish(params).await;
    }
}



async fn sync_address(conn: &Connection) -> Result <()> {
    conn.execute("DELETE from federation_address", ()).expect("Error delete records into address table");
    let height_mainchain = get_pool_info("bitcoin".to_string(), &conn,"blockheight".to_string());
    if let Err(err) = height_mainchain {
        return Ok(());
    }

    let mut node_days = get_bitcoin_block_height_by_day(conn);
    for x in 0..10 {
        let _ = create_new_account((node_days % 1000) as u32, &conn);
        node_days = node_days - 1;
        if node_days<0 {
            break;
        }
    }
    Ok(())
}

async fn reset_peer_status(conn: &Connection) -> Result <()> {
    conn.execute(
        "UPDATE federation_member SET status='inactive'",
        (),
     )?;
     let _ = update_peer_status(env!("PEER_PUB").to_owned(), "active".to_string(), conn);
    Ok(())
}

async fn clear_peg_queue(conn: &Connection) -> Result <()> {
    conn.execute(
        "DELETE from federation_peg",
        (),
     )?;
    Ok(())
}

async fn reset_admin_peer(conn: &Connection) -> Result <()> {
    conn.execute(
        "UPDATE federation_options name='admin_peer' SET value=''",
        (),
     )?;
     Ok(())
}
/**
 * option table write and read functions
 */

 pub fn get_option_value(conn: &Connection, name: String) -> Result<String> {
    let mut stmt = conn.prepare("SELECT value FROM federation_options WHERE name = :name")?;

    let option_list_iter = stmt.query_map(&[(":name", &name)], |row| {
        Ok(OptionList {
            value: row.get(0)?
        })
    })?;
    
    let mut option_value = "".to_string();
    for option_list in option_list_iter {
        let option_value_str = option_list.unwrap().value;
        option_value = option_value_str;
    }

    return Ok(option_value);
}

pub fn set_option_value(value: String, name: String, conn: &Connection) -> Result <()> {
    conn.execute(
        "UPDATE federation_options SET value=?1 WHERE name=?2",
        (&value, &name),
     )?;
   Ok(())
}


pub fn get_api_base(node_type: String) ->  String {
    let mut easy_url = env!("MARANODE_RPC").to_owned();
    if node_type == "bitcoin" {
        easy_url = env!("BITCOIN_RPC").to_owned();
    }
    return easy_url
}