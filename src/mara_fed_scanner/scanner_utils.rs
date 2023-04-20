use curl::easy::Easy;
use rusqlite::{Connection, Result };
use serde_json::from_str;
use crate::mara_fed_wallet::wallet_utils::get_option_value;

use super::{scanner_model::{WalletAddress, TransactionResponse, TransactionType, StatusType, TransactionModel, VinModel, PegModel, DepositModel, DepositList, TotalCount }};
use std::thread;

pub fn listen_zmq() ->  Result<(), zmq::Error>  {
    let conn = Connection::open(env!("DATABASE").to_owned()).unwrap();
    let transaction_mode  =  TransactionType::DEPOSIT.to_string();
    let ctx = zmq::Context::new();
    let socket = ctx.socket(zmq::SUB).unwrap();
    let zmq_url = env!("MAINCHAIN_ZMQ_URL").to_owned();
    let mainchain_rpc = env!("BITCOIN_RPC").to_owned();
    socket.connect(&zmq_url).unwrap();
    socket.set_subscribe(b"hashtx").unwrap();      
    println!("Subscribed.. Waiting for deposit messages.");
    thread::spawn(move || {
        loop {
            let data = socket.recv_multipart(0).unwrap();
            let param2 = &data[1];
            let txhash = hex::encode(param2);
            println!("deposit txhash: {}", txhash);   
            get_transaction_details(&transaction_mode,txhash,&conn,&mainchain_rpc);
        }
    });
    Ok(())
}

pub fn listen_sidechain_zmq() ->  Result<(), zmq::Error>  {
    let conn = Connection::open(env!("DATABASE").to_owned()).unwrap();
    let transaction_mode = TransactionType::WITHDRAW.to_string();
    let ctx = zmq::Context::new();
    let socket = ctx.socket(zmq::SUB).unwrap();
    let zmq_url = env!("SIDECHAIN_ZMQ_URL").to_owned();
    let sidechain_rpc = env!("MARANODE_RPC").to_owned();
    socket.connect(&zmq_url).unwrap();
    socket.set_subscribe(b"hashtx").unwrap();
    println!("Subscribed.. Waiting for withdraw messages.");
    thread::spawn(move || {
        loop {
            let data = socket.recv_multipart(0).unwrap();
            let param2 = &data[1];
            let txhash = hex::encode(param2);
            println!("withdraw txhash: {}", txhash);        
            get_transaction_details(&transaction_mode,txhash,&conn,&sidechain_rpc);
           }
        });
    Ok(())
}

pub fn get_walletaddresses() -> Result <Vec<String>> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut stmt = conn.prepare("SELECT mutlisig_address FROM federation_address")?;
    let account_list_iter = stmt.query_map([], |row| {
        Ok(WalletAddress {
            address: row.get(0)?
        })
    })?;
    let mut tokens = Vec::new();
    for account in account_list_iter {
        tokens.push(account.unwrap().address);
    }
   return Ok(tokens);
}

pub fn get_transaction_details(tx_type: &String,txhash: String,conn: &Connection, rpc: &String) -> Result <()> {
    println!("tx_type {:?}",tx_type);
    let mut output = String::new();
    let max_confirmations = env!("BLOCK_CONFIRNATION_THRESHOLD").parse().unwrap();
    let mut block_height_name = "sidechain_block_height".to_string();
    if rpc == &env!("BITCOIN_RPC").to_owned() {
        block_height_name = "bitcoin_block_height".to_string();
    }

    let mut transaction_status = StatusType::Start;
    let data = ::serde_json::json!({
            "jsonrpc": "1.0",
            "id": "curltest",
            "method": "getrawtransaction",
            "params": [&txhash,1]
    })
    .to_string();
    let mut easy = Easy::new();
    easy.url(&rpc).unwrap();
    easy.post(true).unwrap();
    easy.post_fields_copy(data.as_bytes()).unwrap();
    { // Use the Rust-specific `transfer` method to allow the write function to borrow `output` temporarily
        let mut transfer = easy.transfer();
        transfer.write_function(|data| {
            output.push_str(&String::from_utf8_lossy(data));
            Ok(data.len())
        }).unwrap();
        // Actually execute the request
        let api_respose = transfer.perform();
        if let Err(err) = api_respose {
            println!("Member node connetion failure {:?}",err);
            return Ok(());
        } else {
            api_respose.unwrap();
        }
    }
    let object: TransactionResponse = serde_json::from_str(&output).unwrap();
    let mut confirmations = 0;
    let details= object.result.vout;
    let txid= object.result.txid;
    let vin_details= object.result.vin;
    let vin_txid_array = vin_details[0].clone();
    if vin_txid_array["txid"].is_null() {
        return Ok(());
    }
    let vin_txid = vin_txid_array["txid"].as_str().unwrap();

    if tx_type == &TransactionType::DEPOSIT.to_string() || tx_type == &TransactionType::WITHDRAW.to_string() {
        let iter = details.iter();
        let vin_iter = vin_details.iter();
        let mut receiver_addressess = Vec::new();    
        let check_transaction = check_transaction_exists(tx_type,txhash, &conn).unwrap();  
        if check_transaction == 0 {
            println!("check transaction type {:?}",tx_type);
            if tx_type == &TransactionType::DEPOSIT.to_string() {
                println!("inside deposit ");
                let addresses = get_walletaddresses().unwrap();
                let mut addr_iter = addresses.iter();  
                print!("addr_iter {:?}",addr_iter);
                for detail in iter{
                print!("for inn");
                    let datas = detail.clone();
                    let scriptPubKey_address = datas["scriptPubKey"]["address"].as_str().unwrap().to_owned();
                    receiver_addressess.push(scriptPubKey_address.clone());    
                    let receiver_address = scriptPubKey_address.clone();    
                    let checkforaddress = addresses.contains(&receiver_address);
                    println!("check for receiver_address {:?} {:?} \n", checkforaddress,receiver_address);
                    if checkforaddress == true { 
                        println!("address found ");
                        let get_sender_address = get_sender_address(&vin_txid.to_string(),&conn,&rpc); 
                        let sender_address = get_sender_address.unwrap();
                        let amount = datas["value"].as_f64().unwrap();
                        let mut block_height = from_str::<u32>(&get_option_value(&conn,block_height_name.to_string()).unwrap()).unwrap();
                        block_height = block_height + 1;
                        let _ = save_transaction(tx_type,sender_address.to_string(),receiver_address.to_string(),block_height,txid.to_string(),amount,transaction_status.to_string(),&conn);
                    }           
                }  
            }
            else{
                println!("inside withdraw ");
                let withdraw_address =  env!("WITHDRAW_ADDRESS").to_owned();    
                for detail in iter{
                    let datas = detail.clone();
                    let scriptPubKey_address = datas["scriptPubKey"]["address"].as_str().unwrap().to_owned();
                    receiver_addressess.push(scriptPubKey_address.clone());    
                    let receiver_address = scriptPubKey_address.clone(); 
                    if receiver_address == withdraw_address {
                        let get_sender_address = get_sender_address(&vin_txid.to_string(),&conn,&rpc); 
                        let sender_address = get_sender_address.unwrap();
                        let amount = datas["value"].as_f64().unwrap();
                        println!("sender_address {:?}",sender_address);
                        let mut block_height = from_str::<u32>(&get_option_value(&conn,block_height_name.to_string()).unwrap()).unwrap();
                        block_height = block_height + 1;
                        let _ = save_transaction(tx_type,sender_address.to_string(),receiver_address.to_string(),block_height,txid.to_string(),amount,transaction_status.to_string(),&conn);
                    }                
                }      
            }
        }
        else {
            confirmations = object.result.confirmations.unwrap_or_default();
            if confirmations > max_confirmations {
                transaction_status = StatusType::Completed;
            } else {
                transaction_status = StatusType::Inprogress;
            }
           let _ = update_transaction(tx_type,txid,transaction_status.to_string(), &conn);  
        } 
    } else {
        confirmations = object.result.confirmations.unwrap_or_default();
        if confirmations > max_confirmations {
            transaction_status = StatusType::Completed;
            let _ = update_transaction(tx_type,txid,transaction_status.to_string(), &conn);  
        } 
    }


  Ok(())
}

fn save_transaction(tx_type: &String,sender_address: String,receiver_address: String,block_height: u32, txhash: String, amount: f64,status: String,conn: &Connection) -> Result <(),rusqlite::Error> {
    println!("Saving transaction..{:?}",tx_type);
    if tx_type == "DEPOSIT" { 
        conn.execute(
            "INSERT INTO federation_deposits (sender_address, deposit_address,tx_id,amount,block_height,status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (&sender_address.to_string(),&receiver_address.to_string(),&txhash.to_string(),&amount, &block_height, &status),
        ).expect("Error inserting records into database");        
    }
    else { 
        conn.execute(
            "INSERT INTO federation_withdraw (sender_address, receiver_address,tx_id,amount,block_height,status) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (&sender_address.to_string(),&receiver_address.to_string(),&txhash.to_string(),&amount, &block_height, &status),
        ).expect("Error inserting records into database"); 
    }
    Ok(())
}

fn update_transaction(tx_type: &String, txhash: String, status: String, conn: &Connection) -> Result <(),rusqlite::Error> {
    println!("updating transaction.. {:?}",tx_type);
    if tx_type ==  "DEPOSIT" { 
        conn.execute(
            "UPDATE federation_deposits SET status=?2 WHERE tx_id=?1",
            (txhash, &status),
         ).expect("Error when updating records into federation_deposits table");  
    } else if tx_type ==  &TransactionType::MAINTRANSFER.to_string() {  
        conn.execute(
            "UPDATE federation_deposits SET peg_status=?1 WHERE peg_tx_id=?2",
            (&status, txhash),
         ).expect("Error when updating records into federation_deposits table");  
    } else if tx_type ==  &TransactionType::PEGIN.to_string() {  
        conn.execute(
            "UPDATE federation_peg SET status=?1 WHERE tx_id=?2 AND object_type=?3",
            (&status, txhash, &"0".to_string()),
         )?;  
    } else if tx_type ==  &TransactionType::PEGOUT.to_string() { 
        conn.execute(
            "UPDATE federation_peg SET status=?1 WHERE tx_id=?2 AND object_type=?3",
            (&status, txhash, &"1".to_string()),
         )?;  
    } else{
        conn.execute(
            "UPDATE federation_withdraw SET status=?2 WHERE tx_id=?1",
            (txhash, &status),
         ).expect("Error when updating records into federation_withdraw table");  
    }        
    Ok(())
}

fn check_transaction_exists(tx_type: &String,txhash: String, conn: &Connection) -> Result <u32,rusqlite::Error> {
    println!("Checking transaction..!");
    let mut transactions = Vec::new();
    let mut query  = String::new();
    if tx_type == "DEPOSIT" {
        query = "SELECT tx_id FROM federation_deposits WHERE tx_id = :tx_id".to_string();
    }else{
        query = "SELECT tx_id FROM federation_withdraw WHERE tx_id = :tx_id".to_string();
    }
        let mut stmt = conn.prepare(&query)?; 
        let trans_exist_iter = stmt.query_map(&[(":tx_id", &txhash.to_string())], |row| {
        Ok(TransactionModel {
            txid: row.get(0)?,
            confirmations: Some(0),
            vout: vec![],
            vin: vec![]
        })
    })?;
    for account in trans_exist_iter {
        transactions.push(account.unwrap().txid);        
    }
    if transactions.len() == 0 {
        return Ok(0)
    }
    else {
        return Ok(1)
    }
}

pub fn check_and_process(conn: &Connection) -> Result <()> {
    println!("Check & process deposit tranaction");
    let start = StatusType::Start;
    let inprogress = StatusType::Inprogress;
    let mainchain_rpc = env!("BITCOIN_RPC").to_owned();
    let sidechain_rpc = env!("MARANODE_RPC").to_owned();
    // Check for deposit transactions, iterate & update
    let mut deposit_stmt = conn.prepare("SELECT tx_id,block_height FROM federation_deposits where status=:start or status=:inprogress;")?;
    let deposit_list_iter = deposit_stmt.query_map(&[(":start", &start.to_string()),(":inprogress", &inprogress.to_string())], |row| {
        Ok(TransactionModel {
            txid: row.get(0)?,
            confirmations: row.get(1)?,
            vout: vec![],
            vin: vec![]
        })
    })?;   
    for deposit_item in deposit_list_iter {
        let check_obj: TransactionModel = deposit_item.unwrap();
        let _ = get_transaction_details(&TransactionType::DEPOSIT.to_string(), check_obj.txid,&conn,&mainchain_rpc);
    }  
    
    // Check for withdraw transactions, iterate & update
    let mut withdraw_stmt = conn.prepare("SELECT tx_id,block_height FROM federation_withdraw where status=:start or status=:inprogress;")?;
    let withdraw_list_iter = withdraw_stmt.query_map(&[(":start", &start.to_string()),(":inprogress", &inprogress.to_string())], |row| {
        Ok(TransactionModel {
            txid: row.get(0)?,
            confirmations: row.get(1)?,
            vout: vec![],
            vin: vec![]
        })
    })?;   
    for withdraw_item in withdraw_list_iter {
        let check_obj: TransactionModel = withdraw_item.unwrap();
        let _ = get_transaction_details(&TransactionType::WITHDRAW.to_string(), check_obj.txid,&conn,&sidechain_rpc);
     }

     // check main account deposit
     let _ = check_confirmation_main_account(&conn, &mainchain_rpc);

     // check peg in confirmation
     let _ = check_confirmation_peg_account("0".to_string(),&conn, &sidechain_rpc);

     // check peg out confirmation
     let _ = check_confirmation_peg_account("1".to_string(),&conn, &mainchain_rpc);

     let _ = peg_update_database(&conn);

  Ok(())
}

fn check_confirmation_main_account(conn: &Connection, rpc: &String) -> Result <()>{
    println!("checking from main account sync");
    let completed = StatusType::Completed;
    let mut deposit_stmt = conn.prepare("SELECT peg_tx_id,peg_block_height FROM federation_deposits where peg_status!=:completed AND status=:completed AND pegin=1")?;
    let deposit_list_iter = deposit_stmt.query_map(&[(":completed", &completed.to_string())], |row| {
        Ok(TransactionModel {
            txid: row.get(0)?,
            confirmations: row.get(1)?,
            vout: vec![],
            vin: vec![]
        })
    })?;  

    for deposit_item in deposit_list_iter {
        println!("checking from main account sync {:?}",deposit_item);
        let check_obj: TransactionModel = deposit_item.unwrap();
        let _ = get_transaction_details(&TransactionType::MAINTRANSFER.to_string(), check_obj.txid,&conn,&rpc);
    } 
    Ok(())
}

fn check_confirmation_peg_account(peg_type:String, conn: &Connection, rpc: &String) -> Result <()>{
    let inprogress = StatusType::Inprogress;
    let mut withdraw_stmt = conn.prepare("SELECT tx_id,block_height FROM federation_peg where status=:inprogress AND object_type=:peg_type;")?;
    let withdraw_list_iter = withdraw_stmt.query_map(&[(":inprogress", &inprogress.to_string()),(":peg_type", &peg_type.to_string())], |row| {
        Ok(TransactionModel {
            txid: row.get(0)?,
            confirmations: row.get(1)?,
            vout: vec![],
            vin: vec![]
        })
    })?;   
    let mut transaction_type = TransactionType::PEGIN;
    if peg_type == "pegout" {
        transaction_type = TransactionType::PEGOUT;
    }
    for withdraw_item in withdraw_list_iter {
        let check_obj: TransactionModel = withdraw_item.unwrap();
        let _ = get_transaction_details(&transaction_type.to_string(), check_obj.txid,&conn,&rpc);
     }
   Ok(())
}


pub fn get_sender_address(vintxhash: &String,conn: &Connection, rpc: &String) -> Result<String> {
    println!("Find sender address");
    let mut output = String::new();
    let data = ::serde_json::json!({
        "jsonrpc": "1.0",
        "id": "curltest",
        "method": "getrawtransaction",
        "params": [&vintxhash,1]
    })
    .to_string();
    let mut easy = Easy::new();
    easy.url(&rpc).unwrap();
    easy.post(true).unwrap();
    easy.post_fields_copy(data.as_bytes()).unwrap();
    { // Use the Rust-specific `transfer` method to allow the write function to borrow `output` temporarily
        let mut transfer = easy.transfer();
        transfer.write_function(|data| {
            output.push_str(&String::from_utf8_lossy(data));
            Ok(data.len())
        }).unwrap();
        // Actually execute the request
        transfer.perform().unwrap();
    }
    let object: TransactionResponse = serde_json::from_str(&output).unwrap();
    let vout_details= object.result.vout;
    let datas = vout_details[0].clone();
    let scriptPubKey_address = datas["scriptPubKey"]["address"].as_str().unwrap().to_owned();
    let sender_address = scriptPubKey_address.clone(); 
    Ok(sender_address)
}


fn peg_update_database(conn: &Connection) -> Result<()> {
    let peg_in_last_index = peg_get_last_index(conn, 0).unwrap();
    let peg_out_last_index = peg_get_last_index(conn, 1).unwrap();

    println!("peg_in_last_index {:?}", peg_in_last_index);
    println!("peg_out_last_index {:?}", peg_out_last_index);
     
    let _ = check_and_process_peg_in(conn,peg_in_last_index);
    let _ = check_and_process_peg_out(conn,peg_out_last_index);
    Ok(())
}

fn peg_get_last_index(conn: &Connection, object_type: u32) -> Result<u32> {
    // peg in 
    let mut stmt = conn.prepare("SELECT id,receiver_address,tx_id,amount,block_height,object_id,object_type FROM federation_peg WHERE object_type=:object_type ORDER BY object_id DESC LIMIT 1")?;
    let account_list_iter = stmt.query_map(&[(":object_type", &object_type.to_string())], |row| {
        Ok(PegModel {
            id: row.get(0)?,
            receiver_address: row.get(1)?,
            tx_id: row.get(2)?,
            amount: row.get(3)?,
            confirmations: row.get(4)?,
            object_id: row.get(5)?,
            object_type: row.get(6)?
        })
    })?;

    let mut peg_in_data = Vec::new();
    for account in account_list_iter {
        peg_in_data.push(account.unwrap());
    }
    let mut peg_object_id = 0;
    if peg_in_data.len() > 0 {
        peg_object_id = peg_in_data[0].object_id
    }

    Ok(peg_object_id)
}

fn check_and_process_peg_in(conn: &Connection,object_id:u32) -> Result<()> {
    let completed = StatusType::Completed;
    println!("==========");
    println!("object_idobject_idobject_id {:?}",object_id);
    let mut stmt = conn.prepare("SELECT id, sender_address, tx_id, amount, block_height FROM federation_deposits WHERE id>:id AND peg_status=:completed")?;
    let account_list_iter = stmt.query_map(&[(":id", &object_id.to_string()),(":completed",&completed.to_string())], |row| {
        Ok(DepositModel {
            id: row.get(0)?,
            receiver_address: row.get(1)?,
            tx_id: row.get(2)?,
            amount: row.get(3)?,
            confirmations: row.get(4)?,
        })
    })?;
    let mut block_height = from_str::<u32>(&get_option_value(&conn,"sidechain_block_height".to_string()).unwrap()).unwrap();
    block_height = block_height + 1;

    let transaction_status = StatusType::Start;
    for account in account_list_iter {
        println!("account account {:?}",account);
        let account_item = account.unwrap();
        conn.execute(
            "INSERT INTO federation_peg (receiver_address, tx_id, amount, block_height, object_id, object_type, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (&account_item.receiver_address.to_string(),&account_item.tx_id.to_string(),&account_item.amount.to_string(),&block_height.to_string(), &account_item.id.to_string(),&"0".to_string(), &transaction_status.to_string()),
        ).expect("Error inserting records into database");  
    }

    Ok(())
}

fn check_and_process_peg_out(conn: &Connection,object_id:u32) -> Result<()> {
    let completed = StatusType::Completed;
    let mut stmt = conn.prepare("SELECT id, sender_address, tx_id, amount, block_height  FROM federation_withdraw WHERE id>:id AND status=:completed")?;
    let account_list_iter = stmt.query_map(&[(":id", &object_id.to_string()),(":completed", &completed.to_string())], |row| {
        Ok(DepositModel {
            id: row.get(0)?,
            receiver_address: row.get(1)?,
            tx_id: row.get(2)?,
            amount: row.get(3)?,
            confirmations: row.get(4)?,
        })
    })?;
    let mut block_height = from_str::<u32>(&get_option_value(&conn,"bitcoin_block_height".to_string()).unwrap()).unwrap();
    block_height = block_height + 1;

    let transaction_status = StatusType::Start;
    for account in account_list_iter {
        let account_item = account.unwrap();
        conn.execute(
            "INSERT INTO federation_peg (receiver_address, tx_id, amount, block_height, object_id, object_type, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            (&account_item.receiver_address.to_string(),&account_item.tx_id.to_string(),&account_item.amount.to_string(),block_height.to_string(), &account_item.id.to_string(),&"1".to_string(), &transaction_status.to_string()),
        ).expect("Error inserting records into database");  
    }

    Ok(())
}

pub fn get_deposits_list(search_term: String,limit: usize,offset: usize) -> Result <Vec<DepositList>> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    println!("limit {:?} offset {:?}",limit, offset);
    if search_term.is_empty()
    {
        let mut stmt = conn.prepare("SELECT id, sender_address, deposit_address, tx_id, amount, block_height, status, created_date FROM federation_deposits where status != :status ORDER BY created_date DESC LIMIT :limit OFFSET :offset")?;
        //println!("stmt {:?}",stmt);
        let deposit_list_iter = stmt.query_map(&[(":limit", &limit.to_string().as_str()),(":offset", &offset.to_string().as_str()),(":status", &StatusType::Completed.to_string().as_str())], |row| {
            Ok(DepositList {
            id: row.get(0)?,
            sender_address: row.get(1)?,
            receiver_address: row.get(2)?,            
            txid: row.get(3)?,
            amount: row.get(4)?,
            confirmations: row.get(5)?,
            status: row.get(6)?,
            date: row.get(7)?
            })
        })?;
        let mut data = Vec::new();
        for item in deposit_list_iter {
            data.push(item.unwrap());
        }
        return Ok(data);
    }
    else{
        let percentage_symbol = "%".to_string();
        let search_text = search_term.replace("\"", "");
        let pattern_string = percentage_symbol.clone()+&search_text+&percentage_symbol.clone();
        println!("search_text {:?}",pattern_string);
        let mut stmt = conn.prepare("SELECT id, sender_address, deposit_address, tx_id, amount, block_height, status, created_date FROM federation_deposits where status != :status AND (sender_address LIKE :search_term OR deposit_address LIKE :search_term OR tx_id LIKE :search_term ) ORDER BY created_date DESC LIMIT :limit OFFSET :offset ")?;
        println!("stmt {:?}",stmt);
        let deposit_list_iter = stmt.query_map(&[(":search_term", &pattern_string),(":limit", &limit.to_string()),(":offset", &offset.to_string()),(":status", &StatusType::Completed.to_string())], |row| {
            Ok(DepositList {
            id: row.get(0)?,
            sender_address: row.get(1)?,
            receiver_address: row.get(2)?,            
            txid: row.get(3)?,
            amount: row.get(4)?,
            confirmations: row.get(5)?,
            status: row.get(6)?,
            date: row.get(7)?
            })
        })?;
        let mut data = Vec::new();
        for item in deposit_list_iter {
            data.push(item.unwrap());
        }
        return Ok(data);
    }
}

pub fn get_withdraws_list(search_term: String,limit: usize,offset: usize) -> Result <Vec<DepositList>> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    println!("limit {:?} offset {:?}",limit, offset);
    if search_term.is_empty()
    {
        let mut stmt = conn.prepare("SELECT id, sender_address, receiver_address, tx_id, amount, block_height, status, created_date FROM federation_withdraw where status != :status ORDER BY created_date DESC LIMIT :limit OFFSET :offset")?;
         let deposit_list_iter = stmt.query_map(&[(":limit", &limit.to_string().as_str()),(":offset", &offset.to_string().as_str()),(":status", &StatusType::Completed.to_string().as_str())], |row| {
            Ok(DepositList {
            id: row.get(0)?,
            sender_address: row.get(1)?,
            receiver_address: row.get(2)?,            
            txid: row.get(3)?,
            amount: row.get(4)?,
            confirmations: row.get(5)?,
            status: row.get(6)?,
            date: row.get(7)?
            })
        })?;
        let mut data = Vec::new();
        for item in deposit_list_iter {
            data.push(item.unwrap());
        }
        return Ok(data);
    }
    else{
        let percentage_symbol = "%".to_string();
        let search_text = search_term.replace("\"", "");
        let pattern_string = percentage_symbol.clone()+&search_text+&percentage_symbol.clone();
        let mut stmt = conn.prepare("SELECT id, sender_address, receiver_address, tx_id, amount, block_height, status, created_date FROM federation_withdraw where status != :status AND (sender_address LIKE :search_term OR receiver_address LIKE :search_term OR tx_id LIKE :search_term ) ORDER BY created_date DESC LIMIT :limit OFFSET :offset ")?;
        //println!("stmt {:?}",stmt);
        let deposit_list_iter = stmt.query_map(&[(":search_term", &pattern_string),(":limit", &limit.to_string()),(":offset", &offset.to_string()),(":status", &StatusType::Completed.to_string())], |row| {
            Ok(DepositList {
            id: row.get(0)?,
            sender_address: row.get(1)?,
            receiver_address: row.get(2)?,            
            txid: row.get(3)?,
            amount: row.get(4)?,
            confirmations: row.get(5)?,
            status: row.get(6)?,
            date: row.get(7)?
            })
        })?;
        let mut data = Vec::new();
        for item in deposit_list_iter {
            data.push(item.unwrap());
        }
        return Ok(data);
    }
}

pub fn get_deposits_count(search_term: String) -> Result <u32> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut transactions = Vec::new();
  
    if search_term.is_empty(){
        let mut stmt = conn.prepare("SELECT COUNT(tx_id) AS cnt FROM federation_deposits where status != :status")?; 
        let list_iter = stmt.query_map(&[(":status", &StatusType::Completed.to_string())], |row| {
                Ok(TotalCount {
                    cnt: row.get(0)?
                })
            })?; 
        for account in list_iter {
            transactions.push(account.unwrap().cnt);        
        }
        println!("transactions {:?}",transactions[0]);
        let total_records = transactions[0];
        return Ok(total_records)
    }else{
        let percentage_symbol = "%".to_string();
        let search_text = search_term.replace("\"", "");
        let pattern_string = percentage_symbol.clone()+&search_text+&percentage_symbol.clone();
        let mut stmt = conn.prepare("SELECT COUNT(tx_id) AS cnt FROM federation_deposits where status != :status AND (sender_address LIKE :search_term OR deposit_address LIKE :search_term OR tx_id LIKE :search_term )")?; 
        let list_iter = stmt.query_map(&[(":status", &StatusType::Completed.to_string()),(":search_term", &pattern_string)], |row| {
                Ok(TotalCount {
                    cnt: row.get(0)?
                })
            })?; 
        for account in list_iter {
            transactions.push(account.unwrap().cnt);        
        }
        println!("transactions {:?}",transactions[0]);
        let total_records = transactions[0];
        return Ok(total_records)
    }   
}


pub fn get_withdraws_count(search_term: String) -> Result <u32> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut transactions = Vec::new();
   
    if search_term.is_empty(){
        let mut stmt = conn.prepare("SELECT COUNT(tx_id) AS cnt FROM federation_withdraw where status != :status")?; 
        let list_iter = stmt.query_map(&[(":status", &StatusType::Completed.to_string())], |row| {
                Ok(TotalCount {
                    cnt: row.get(0)?
                })
            })?; 
        for account in list_iter {
            transactions.push(account.unwrap().cnt);        
        }
        println!("transactions {:?}",transactions[0]);
        let total_records = transactions[0];
        if total_records == 0 {
            return Ok(0)
        }
        else {
            return Ok(total_records)
        }
    }else{
        let percentage_symbol = "%".to_string();
        let search_text = search_term.replace("\"", "");
        let pattern_string = percentage_symbol.clone()+&search_text+&percentage_symbol.clone();
        let mut stmt = conn.prepare("SELECT COUNT(tx_id) AS cnt FROM federation_withdraw where status != :status AND (sender_address LIKE :search_term OR receiver_address LIKE :search_term OR tx_id LIKE :search_term )")?; 
        let list_iter = stmt.query_map(&[(":status", &StatusType::Completed.to_string()),(":search_term", &search_text)], |row| {
                Ok(TotalCount {
                    cnt: row.get(0)?
                })
            })?; 
        for account in list_iter {
            transactions.push(account.unwrap().cnt);        
        }
        println!("transactions {:?}",transactions[0]);
        let total_records = transactions[0];
        if total_records == 0 {
            return Ok(0)
        }
        else {
            return Ok(total_records)
        }
    }   
}


