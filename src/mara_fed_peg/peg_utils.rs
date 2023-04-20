use curl::easy::Easy;
use rusqlite::{Connection, Result };
use crate::{mara_fed_member::member_utils::{FedMember, get_members, NodeMember}, mara_fed_wallet::wallet_utils::{PubkeyResponse, get_pub_key_from_cli, generate_deposit_address, is_decision_maker, WalletList, get_option_value}, mara_fed_scanner::{scanner_model::{StatusType, WalletAddress}, scanner_utils::check_and_process}, mara_cli::{mara_cli_dto::RedeemRequestDTO, mara_cli_service::execute_cli}, mara_fed_transaction::{transaction_utils::{generate_transaction, sign_from_cli, finalize_transaction_from_cli, submit_tx_to_rpc, save_tx_history}, transaction_dto::SignDTO, transaction_model::SignModel}};
use serde_json::{from_str};


use super::peg_model::{DepositInfoModel, SpenderTreePaths};

pub fn get_master_details(conn: &Connection) -> (Vec<String>,String, Vec<NodeMember>) {

    let mut addresslist = Vec::new();
    let members = get_members(conn,false).unwrap();
    let mut nodes = Vec::new();
    for member in members {
        let cli_response = get_pub_key_from_cli(&0, member.master_pub.to_string()).unwrap();
        nodes.push(NodeMember {
            is_own: if member.identity.to_string() != env!("IDENTITY").to_owned() {false} else {true},
            member: member,
            pub_key: cli_response.to_string()
        });
        addresslist.push(cli_response.to_string());
    }

    let generate_result = generate_deposit_address(addresslist.to_vec());
    let newaddress: String = generate_result.unwrap();
    if newaddress != "".to_string() {
        println!("newaddress {:?}",newaddress);
    }
    return (addresslist, newaddress, nodes);
}


pub fn get_user_address(indexer:&u32, conn: &Connection) -> Result<(Vec<String>, Vec<NodeMember>)> {
    let mut addresslist = Vec::new();
    let members = get_members(conn, false).unwrap();
    let mut nodes = Vec::new();
    for member in members {
        let cli_response = get_pub_key_from_cli(&indexer, member.user_pub.to_string()).unwrap();
        nodes.push(NodeMember {
            is_own: if member.identity.to_string() != env!("IDENTITY").to_owned() {false} else {true},
            member: member,
            pub_key: cli_response.to_string()
        });
        addresslist.push(cli_response.to_string())
    }
    Ok((addresslist,nodes))
}


pub fn check_initailize_peg(conn: &Connection) {
    let decision = is_decision_maker(conn);
    if decision.0 {
        initialize_peg(conn);
    } 
}

pub fn tick_confirmation(conn: &Connection) {
    let _ : Result<(), rusqlite::Error> = check_and_process(&conn);  
}
  
pub fn initialize_peg(conn: &Connection) {
    println!("================");
    println!("STARTED  ");
    println!("================"); 
    let _ = process_master_transaction(&conn);
    let _ = process_peg_transaction(&conn, "0".to_string());
    let _ = process_peg_transaction(&conn, "1".to_string());
}

fn process_master_transaction(conn: &Connection) -> Result<()> {
   println!("finding master details to receiving amount");
   let master_details = get_master_details(conn); 
   let master_address = master_details.1;

   println!("finding next address which receive deposit");
    let deposit_address_result = find_next_address_master(&conn).unwrap();
    if deposit_address_result.len() == 0 {
        return Ok(());
    }

    println!("finding index of deposit address");
    let address_details = find_address_by_index(conn,&deposit_address_result[0].address);
    if let Err(err) = address_details {
        return Ok(());
    }
    let indexer = &address_details.unwrap().indexer;

    println!("extract all public keys");
    let get_user_address_result = get_user_address(indexer, &conn).unwrap();
    let pubkeys = get_user_address_result.0;
    let signing_nodes = get_user_address_result.1;


    let redeem_paths_result_str = generate_spender_tree(&pubkeys).unwrap();
    let mut redeem_paths_result = serde_json::from_str::<Vec<SpenderTreePaths>>(&redeem_paths_result_str).unwrap();

    if &redeem_paths_result.len() > &0 {
        redeem_paths_result = filter_spend_tree(&redeem_paths_result, &signing_nodes).unwrap();
    }

    println!("get redeem paths {:?}",&redeem_paths_result);
   
    if &redeem_paths_result.len() == &0 {
        return Ok(());
    }

    println!("get all received transaction for particular address");
    let received_transactions_result = pick_pending_deposit_list_master(conn, &deposit_address_result[0].address);
    let received_transactions = received_transactions_result.unwrap();



    println!("prepare params for generate transaction");
    let mut deposit_address_vec = Vec::new();
    deposit_address_vec.push(deposit_address_result[0].address.to_string());

    let mut receiver_address_vec = Vec::new();
    receiver_address_vec.push(master_address.to_string());


    let mut value_total = 0.0 as f64;
    let mut value_total_vec = Vec::new();
    for  received_transaction in  &received_transactions{
        value_total = value_total + received_transaction.amount;
    }
    value_total_vec.push(value_total);
    

    for redeem_paths_result_item in  redeem_paths_result{
        let redeem_path  = redeem_paths_result_item.tree;
        let transaction_hex_result = generate_transaction(deposit_address_vec.to_vec(), receiver_address_vec.to_vec(), value_total_vec.to_vec(), "bitcoin".to_string(), pubkeys.to_vec(), redeem_path.to_vec(),conn);
        if let Err(err) = transaction_hex_result {
            return Ok(());
        }

        let mut transaction_hex = transaction_hex_result.unwrap();
        println!("transaction_hex {:?}",transaction_hex);
        let mut sign_success = true;
        if transaction_hex.to_string() == "" {
            println!("master transaction_hex input have no amount");
            break;
        }

        for redeem_pubkey in redeem_path.to_vec() {
      
            sign_success = false;
            for (i, node_item) in signing_nodes.iter().enumerate() {
                if node_item.pub_key == redeem_pubkey {
                    println!("redeem_pubkey{:?}",redeem_pubkey);
                
                    if node_item.is_own == true{
                        let params = SignDTO {
                            account_type: "user".to_string(),
                            hex: transaction_hex.to_string(),
                            index: indexer.to_string()
                        };
                        let sign_result = sign_from_cli(&params, conn).unwrap();
                        transaction_hex = sign_result;
                        sign_success = true;
                    }  else {
                        let sign_result =  get_sign_from_member(node_item, indexer, &transaction_hex, "user".to_string());
                        if let Err(_err) = &sign_result {
                            println!("{:?}",_err);
                            sign_success = false;
                        } else {
                            sign_success = true;
                        }
                        transaction_hex = sign_result.unwrap()
                    }
                }
      
            }
            if sign_success == false {
                break;
            }
        }

        if sign_success == true {
            let tx_final = finalize_transaction_from_cli(transaction_hex.to_string()).unwrap();
            let tx_hash = submit_tx_to_rpc(tx_final, "bitcoin".to_string()).unwrap();
            if tx_hash !="".to_string() {
                for received_transaction in  &received_transactions {
                    let _ = update_master_deposit_tx(conn, received_transaction.id.to_string(), tx_hash.to_string());
                }
            }
            break;
        }
    }

    Ok(())
}

fn find_next_address_master(conn: &Connection) -> Result<(Vec<WalletAddress>)> {
    let completed = StatusType::Completed;
    let mut deposit_stmt = conn.prepare("SELECT deposit_address FROM federation_deposits where status=:completed AND peg_tx_id=:peg_tx_id AND pegin=0 LIMIT 1")?;
    let deposit_list_iter = deposit_stmt.query_map(&[(":completed", &completed.to_string()),(":peg_tx_id", &"".to_string())], |row| {
        Ok(WalletAddress {
            address: row.get(0)?
        })
    })?; 
    let mut result = Vec::new();
    for deposit_item in deposit_list_iter {
        result.push(deposit_item.unwrap());
    }
    Ok(result)
}

fn find_address_by_index(conn: &Connection, mutlisig_address: &String) -> Result<(WalletList)> {
    let mut stmt = conn.prepare("SELECT mutlisig_address, indexer FROM federation_address WHERE mutlisig_address=:mutlisig_address LIMIT 1")?;

    let account_list_iter = stmt.query_map(&[(":mutlisig_address", &mutlisig_address.to_string())], |row| {
        Ok(WalletList {
            mutlisig_address: row.get(0)?,
            indexer: row.get(1)?
        })
    })?;

    let mut wallets = Vec::new();
    for account in account_list_iter {
        wallets.push(account.unwrap());
    }

    let wallet_item = WalletList {
        mutlisig_address: wallets[0].mutlisig_address.to_string(),
        indexer: wallets[0].indexer as u32
    };
    return Ok(wallet_item);

}

fn pick_pending_deposit_list_master(conn: &Connection, deposit_address: &String) -> Result<Vec<DepositInfoModel>> {
    let completed = StatusType::Completed;
    let mut deposit_stmt = conn.prepare("SELECT id, sender_address, tx_id, amount FROM federation_deposits where status=:completed AND pegin=0 AND deposit_address=:deposit_address LIMIT 10;")?;
    let deposit_list_iter = deposit_stmt.query_map(&[(":completed", &completed.to_string()),(":deposit_address", &deposit_address.to_string())], |row| {
        Ok(DepositInfoModel {
            id: row.get(0)?,
            sender_address: row.get(1)?,
            tx_id: row.get(2)?,
            amount: row.get(3)?
        })
    })?;

    let mut result = Vec::new();
    for deposit_item in deposit_list_iter {
        result.push(deposit_item.unwrap());
    }
    Ok(result)
}

fn update_master_deposit_tx(conn: &Connection, row_id: String, peg_tx_id: String) -> Result<()> {
    let mut block_height = from_str::<u32>(&get_option_value(&conn,"bitcoin_block_height".to_string()).unwrap()).unwrap();
    block_height = block_height + 1;

    conn.execute(
        "UPDATE federation_deposits SET peg_tx_id=?1, peg_block_height=?2, pegin=1 WHERE id=?3",
        (&peg_tx_id, &block_height, &row_id)
    ).expect("Error when updating records into federation_deposits table");
    
    Ok(())
}

fn generate_spender_tree(pubkeys: &Vec<String>) -> Result<String>{
    let request_obj = RedeemRequestDTO {
        pubkeys : pubkeys.to_vec(),
    };
    let params = serde_json::to_string(&request_obj).unwrap();
    let cli_response = execute_cli("-r".to_string(), params);
    if let Err(err) = cli_response {
        println!("error on gemerate redeem path from cli {:?}",err);
        return Ok("".to_string());
    }
    let result = cli_response.unwrap();
    if result.status == false {
        return Ok("".to_string());
    };
    return Ok(result.result);
}

fn filter_spend_tree(spend_tree: &Vec<SpenderTreePaths>, signing_nodes: &Vec<NodeMember>) -> Result<Vec<SpenderTreePaths>>{
    let mut new_spend_tree = Vec::new();
    let online_nodes = get_online_nodes(signing_nodes).unwrap();
    for spend_tree_item in spend_tree {
        let mut is_available = true;
        for pub_key in &spend_tree_item.tree {
            let mut is_node_available = false;
            for online_nodes_item in  online_nodes.to_vec() {
                if online_nodes_item.pub_key == pub_key.to_string() {
                    is_node_available = true;
                    break;
                }
            }
            if !is_node_available  {
               is_available = false;
               break;
            }
        }
        if is_available {
            new_spend_tree.push(SpenderTreePaths { tree: spend_tree_item.tree.to_vec() });
        }
    }

    Ok(new_spend_tree)
}


fn get_online_nodes(signing_nodes: &Vec<NodeMember>) -> Result<Vec<&NodeMember>> {
    let mut new_online_nodes = Vec::new();
    for signing_node_item in signing_nodes {
         if signing_node_item.member.status == "active".to_string() {
            new_online_nodes.push(signing_node_item);
         }
    }
    Ok(new_online_nodes)
}

fn get_sign_from_member(member:&NodeMember, indexer:&u32, hex: &String, account_type: String) -> Result<String> {
    let mut output = String::new();
    let data = ::serde_json::json!({
        "account_type": account_type,
        "index": indexer.to_string(),
        "hex": hex
    }).to_string();
    let urlstr =  "http://".to_string()+ &member.member.host + &":".to_string() + &member.member.port + &"/transaction/sign".to_string();
    let mut easy = Easy::new();
    easy.url(&urlstr).unwrap();
    easy.post(true).unwrap();
    easy.post_fields_copy(data.as_bytes()).unwrap();

    { // Use the Rust-specific `transfer` method to allow the write function to borrow `output` temporarily
        let mut transfer = easy.transfer();

        transfer.write_function(|data| {
            output.push_str(&String::from_utf8_lossy(data));
            Ok(data.len())
        }).unwrap();
        let api_respose = transfer.perform();
        if let Err(err) = api_respose {
            println!("Member node connetion failure {:?}",err);
            return Ok("".to_string());
        } else {
            api_respose.unwrap();
        }
    }
    let object: SignModel = serde_json::from_str(&output).unwrap();
    Ok(object.hex)
    
}

fn process_peg_transaction(conn: &Connection, object_type: String) -> Result<()> {
    println!("initiate depositer");
    let mut node_type = "bitcoin".to_string();
    if object_type == "0" {
        node_type = "maranode".to_string();
    }
    let master_details = get_master_details(conn); 

    let pubkeys = master_details.0;
    let deposit_address = master_details.1;
    let signing_nodes = master_details.2;

    let redeem_paths_result_str = generate_spender_tree(&pubkeys).unwrap();
    let mut redeem_paths_result = serde_json::from_str::<Vec<SpenderTreePaths>>(&redeem_paths_result_str).unwrap();
    
    if &redeem_paths_result.len() > &0 {
        redeem_paths_result = filter_spend_tree(&redeem_paths_result, &signing_nodes).unwrap();
    }


    println!("get redeem paths {:?}",&redeem_paths_result);
   
    if &redeem_paths_result.len() == &0 {
        return Ok(());
    }

    let received_transactions_result = pick_pending_peg_list_master(conn, &object_type.to_string());
    let received_transactions = received_transactions_result.unwrap();
    if &received_transactions.len() == &0 {
        return Ok(());
    }
    println!("get all received petransaction for particular address {:?}",received_transactions);
    let mut deposit_address_vec = Vec::new();
    deposit_address_vec.push(deposit_address.to_string());

    let mut receiver_address_vec = Vec::new();
    let mut value_total = 0.0 as f64;
    let mut value_total_vec = Vec::new();


    for rreceived_transactions_item in  &received_transactions {
        receiver_address_vec.push(rreceived_transactions_item.sender_address.to_string());
        value_total_vec.push(rreceived_transactions_item.amount);
        value_total = value_total + rreceived_transactions_item.amount;
    }

    for redeem_paths_result_item in  redeem_paths_result{
        let redeem_path  = redeem_paths_result_item.tree;
        let transaction_hex_result = generate_transaction(deposit_address_vec.to_vec(), receiver_address_vec.to_vec(), value_total_vec.to_vec(), node_type.to_string(), pubkeys.to_vec(), redeem_path.to_vec(),conn);
        if let Err(err) = transaction_hex_result {
            return Ok(());
        }
        let mut transaction_hex = transaction_hex_result.unwrap();

        let mut sign_success = true;

        if transaction_hex.to_string() == "" {
            println!("peg transaction_hex input have no amount");
            break;
        }
      
        for redeem_pubkey in redeem_path.to_vec() {
            println!("redeem_pubkey{:?}",redeem_pubkey);
            sign_success = false;
            for (i, node_item) in signing_nodes.iter().enumerate() {
                if node_item.pub_key == redeem_pubkey {
                    println!("node_item{:?}",node_item);
                    if node_item.is_own == true{
                        let params = SignDTO {
                            account_type: "master".to_string(),
                            hex: transaction_hex.to_string(),
                            index: "0".to_string()
                        };
                        let sign_result = sign_from_cli(&params, conn).unwrap();
                        transaction_hex = sign_result;
                        sign_success = true;
                    }  else {
                        let sign_result =  get_sign_from_member(node_item, &0 , &transaction_hex, "master".to_string());
                        if let Err(_err) = &sign_result {
                            println!("{:?}",_err);
                            sign_success = false;
                        } else {
                            sign_success = true;
                        }
                        transaction_hex = sign_result.unwrap();
                    }
             
                }
 
      
            }
            if sign_success == false {
                break;
            }
        }

        if sign_success == true {
            let tx_final = finalize_transaction_from_cli(transaction_hex.to_string()).unwrap();
            let tx_hash = submit_tx_to_rpc(tx_final, node_type.to_string()).unwrap();
            if tx_hash !="".to_string() {
                for received_transaction in  &received_transactions {
                    let mut url_type = 2;
                    if object_type == "1".to_string() {
                        url_type = 1
                    }
    
                    let update_master_peg_tx = update_master_peg_tx(conn, received_transaction.id,tx_hash.to_string());
                    if let Err(err) = update_master_peg_tx {
                        println!("update_master_peg_tx {:?}",err)
                    }
                    let _ = save_tx_history(conn, received_transaction.sender_address.to_string(), tx_hash.to_string(), received_transaction.amount.to_string(), "0".to_string(), url_type);
                }
                break;
            }
        }
    }

    Ok(())
}

fn pick_pending_peg_list_master(conn: &Connection, object_type: &String) -> Result<Vec<DepositInfoModel>> {
    let start = StatusType::Start;
    let mut deposit_stmt = conn.prepare("SELECT id, receiver_address, tx_id, amount FROM federation_peg WHERE status=:start AND object_type=:object_type LIMIT 10;")?;
    let deposit_list_iter = deposit_stmt.query_map(&[(":start", &start.to_string()), (":object_type", &object_type.to_string())], |row| {
        Ok(DepositInfoModel {
            id: row.get(0)?,
            sender_address: row.get(1)?,
            tx_id: row.get(2)?,
            amount: row.get(3)?
        })
    })?;

    let mut result = Vec::new();
    for deposit_item in deposit_list_iter {
        result.push(deposit_item.unwrap());
    }
    Ok(result)
}

fn update_master_peg_tx(conn: &Connection, row_id: u32, tx_id: String) -> Result<()> {
    let progress = StatusType::Inprogress;
    println!("============from update peg tx===============");
    println!("{:?}",row_id);
    println!("{:?}",progress.to_string());
    conn.execute(
        "UPDATE federation_peg SET status=?1, tx_id=?2 WHERE id=?3",
        (&progress.to_string(), &tx_id.to_string() ,&row_id)
    )?;
    Ok(())
}

