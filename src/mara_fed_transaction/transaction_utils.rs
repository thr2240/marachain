use curl::easy::Easy;
use std::str::FromStr;
use crate::{mara_fed_transaction::transaction_utils::ResponseError::Null, mara_fed_peg::peg_utils::get_master_details, mara_fed_wallet::wallet_utils::set_decision_maker};
use walletlib::bitcoin::{hashes::hex::FromHex, blockdata::script};
use serde::{Serialize, Deserialize};
use rusqlite::{Result, Connection, NO_PARAMS};
use crate::{mara_fed_wallet::wallet_utils::{get_api_base, get_option_value, set_option_value}, mara_fed_member::member_utils::get_members, mara_cli::{mara_cli_dto::{CLIInputsDTO, CLIOutputsDTO, TransactionRequestDTO, SignatureRequestDTO, FinalizeRequestDTO}, mara_cli_service::execute_cli}};
use walletlib::{
    bitcoin::{
        TxIn, Address, Amount, TxOut, Transaction, Script, Txid, OutPoint
    },
};
use serde_json::from_str;
const RBF: u32 = 0xffffffff - 2;
use super::transaction_dto::SignDTO;


#[derive(Serialize, Deserialize, Debug)]
pub struct BalanceResponse {
    pub result: BalanceModel,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BalanceModel {
    pub height: u32,
    pub total_amount: f64,
    pub unspents: Vec<serde_json::Value>,
}

#[derive(Debug, PartialEq, PartialOrd)]
struct UnspentVec {
    desc: String,
    amount: f64,
    txid: String,
    vout: u32,
}

impl UnspentVec {
    pub fn new(desc: String, amount: f64, txid: String, vout: u32) -> Self {
        UnspentVec {
            desc,
            amount,
            txid,
            vout,
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct UnspentList {
    desc: String,
    amount: f64,
    txid: String,
    vout: u32,
    height: u32,
}


#[derive(Deserialize, Debug)]
pub struct TransactionResponse {
    result: String,
}

#[derive(PartialEq, Deserialize, Debug)]
pub struct TransactionErrorResponse {
    error: ResponseError,
}

#[derive(PartialEq, Deserialize, Debug)]
#[serde(untagged)]
enum ResponseError {
    Message(String),
    CodeMessage { code: i32, message: String },
    Null,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PoolResponse {
    result: PoolModel,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockHeightResponse {
    result: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PoolModel {
    pub mempoolminfee: f64,
}

pub struct SumDeposit {
    value: f64,
}

pub struct SumWithdraw {
    value: f64,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ChartWithdraw {
    xaxis: String,
    value: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChartDeposit {
    xaxis: String,
    value: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FeeData {
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListHistory {
    address: String,
    tx_id: String,
    amount: String,
    fee: String,
    types: i64,
    created_date: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TotalHistory {
    total: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ListAddress {
    mutlisig_address: String,
    indexer: i64,
    created_date: String,
} 



pub fn scan_transaction(address_list:&Vec<String>,url_type: &String) -> Result<BalanceResponse> {
    
    let mut address_param = Vec::new();
    for account in address_list {
        address_param.push( "addr(".to_string() + &account.to_string() + &")".to_string());
    }
 
    let mut output = String::new();
    let data = ::serde_json::json!({
            "jsonrpc": "1.0",
            "id": "curltest",
            "method": "scantxoutset",
            "params": ["start",address_param]
    })
    .to_string();
    let easy_url = get_api_base(url_type.to_string());
    let mut easy = Easy::new();
    easy.url(&easy_url.to_owned()).unwrap();
    easy.post(true).unwrap();
    easy.post_fields_copy(data.as_bytes()).unwrap();
    {
        // Use the Rust-specific `transfer` method to allow the write function to borrow `output` temporarily
        let mut transfer = easy.transfer();

        transfer
            .write_function(|data| {
                output.push_str(&String::from_utf8_lossy(data));
                Ok(data.len())
            })
            .unwrap();

        let api_respose = transfer.perform();
        if let Err(err) = api_respose {
            println!("Main node connection failure {:?}", err);
        } else {
            api_respose.unwrap();
        }
    }
    let object: BalanceResponse = serde_json::from_str(&output).unwrap();
    return Ok(object);
}

pub fn generate_transaction(from_address:Vec<String>, to_address: Vec<String>, value: Vec<f64>, node_type: String, pubkeys: Vec<String>, redeem: Vec<String>, conn: &Connection) -> Result<String> {
    let sum_amount: f64 = value.clone().iter().sum();
    let input_result = prepare_transaction_input(&from_address, sum_amount, &node_type);

    let (input_tr_unwrap, required_amout, input_unspent_data) = input_result;
    let required_amout = format!("{:.10}", required_amout).parse::<f64>().unwrap();

    if &input_tr_unwrap.len() == &0 {
        return Ok("".to_string());
    }


    let mut out_address = to_address.clone();
    let mut out_value = value.clone();
    if required_amout > 0.0 {
        out_address.push(from_address[0].to_string());
        let remaining_balance = required_amout - sum_amount;
        out_value.push(remaining_balance);
    }
    let new_value = reduct_fee_on_values(&input_tr_unwrap, &out_address, &out_value, &node_type, conn).unwrap();

    let generate_result = generate_transation_hex(&input_unspent_data,&out_address,new_value,pubkeys,redeem);

    let psbt_hex: String = generate_result.unwrap();
    
    Ok(psbt_hex)
}

pub fn sign_from_cli(parames: &SignDTO, conn: &Connection) -> Result<String> {
    let peer_id = get_option_value(&conn,"admin_peer".to_string()).unwrap();
    if peer_id == "".to_string() {
        return Ok("".to_string());
    }

    let menomic_value = env!("MNEMONIC").to_owned();

    let request_obj = SignatureRequestDTO {
        index: parames.index.to_string(),
        mnemonic: menomic_value,
        hex: parames.hex.to_string()
    };
    let params = serde_json::to_string(&request_obj).unwrap();

    let cli_response = execute_cli("-s".to_string(), params);
    if let Err(err) = cli_response {
        println!("error on sign from cli {:?}",err);
        return Ok("".to_string());
    }
    let result = cli_response.unwrap();
    if result.status == false {
        return Ok("".to_string());
    };
    return Ok(result.result);
}

pub fn finalize_transaction_from_cli(hex:String) -> Result<String> {
 
    let request_obj = FinalizeRequestDTO {
        hex: hex,
    };
    let params = serde_json::to_string(&request_obj).unwrap();
    let cli_response = execute_cli("-f".to_string(), params);
    if let Err(err) = cli_response {
        println!("error on transaction finalize from cli {:?}",err);
        return Ok("".to_string());
    }
    let result = cli_response.unwrap();
    if result.status == false {
        return Ok("".to_string());
    };
    return Ok(result.result);
}

pub fn submit_tx_to_rpc(tx: String, node_type:String) -> Result<String> {
    let mut output = String::new();
    let data = ::serde_json::json!({
            "jsonrpc": "1.0",
            "id": "curltest",
            "method": "sendrawtransaction",
            "params": [tx]
    })
    .to_string();
    let mut easy_url = &env!("MARANODE_RPC");
    if node_type == "bitcoin" {
        easy_url = &env!("BITCOIN_RPC");
    }
    let mut easy = Easy::new();
    easy.url(easy_url.to_owned()).unwrap();
    easy.post(true).unwrap();
    easy.post_fields_copy(data.as_bytes()).unwrap();
    {
        let mut transfer = easy.transfer();
        transfer
            .write_function(|data| {
                output.push_str(&String::from_utf8_lossy(data));
                Ok(data.len())
            })
            .unwrap();
        let api_respose = transfer.perform();
        if let Err(err) = api_respose {
            println!("node connection failure {:?}", err);
        } else {
            api_respose.unwrap();
        }
    }

    println!("start coming here {:?}", output);
    let object: TransactionErrorResponse = serde_json::from_str(&output).unwrap();
    if object.error == Null {
        let object_success: TransactionResponse = serde_json::from_str(&output).unwrap();
        return Ok(object_success.result.to_string());
    } 
    Ok("".to_string())
}

pub fn save_tx_history(
    conn: &Connection,
    address: String,
    tx_id: String,
    amount: String,
    fee: String,
    url_type: u32
) -> Result<()> {
    conn.execute(
        "INSERT INTO federation_history (address, tx_id, amount, fee, type) VALUES (?1, ?2, ?3,?4, ?5)",
        (&address, &tx_id.to_string(), &amount, &fee, &url_type),
    )?;
    Ok(())
}

pub fn get_pool_info(node_type: String, conn: &Connection, method_type: String) -> Result<()>{
    let mut output = String::new();
    let mut pool_method = "getmempoolinfo".to_string();
    if method_type == "blockheight".to_string() {
        pool_method = "getblockcount".to_string()
    }
    let data = ::serde_json::json!({
            "jsonrpc": "1.0",
            "id": "curltest",
            "method": pool_method,
            "params": []
    })
    .to_string();
    let mut name = "sidechain_min_fee".to_string();
    if method_type == "fee".to_string() {
        if node_type == "bitcoin" {
            name = "bitcoin_min_fee".to_string();
        }
    }

    let mut current_block_height = 0;

    if method_type == "blockheight".to_string() {
        name = "sidechain_block_height".to_string();
        if node_type == "bitcoin" {
            name = "bitcoin_block_height".to_string();
            current_block_height = from_str::<u32>(&get_option_value(&conn,"bitcoin_block_height".to_string()).unwrap()).unwrap();
        }
    }


    let mut easy_url = &env!("MARANODE_RPC");
    if node_type == "bitcoin" {
        easy_url = &env!("BITCOIN_RPC");
    }
    let mut easy = Easy::new();
    easy.url(easy_url.to_owned()).unwrap();
    easy.post(true).unwrap();
    easy.post_fields_copy(data.as_bytes()).unwrap();
    {
        let mut transfer = easy.transfer();
        transfer
            .write_function(|data| {
                output.push_str(&String::from_utf8_lossy(data));
                Ok(data.len())
            })
            .unwrap();

        let api_respose = transfer.perform();
        if let Err(err) = api_respose {
            println!("node connection failure {:?}", err);
            return Ok(());
        } else {
            api_respose.unwrap();
        }
    }
    if method_type == "fee" {
        let object: PoolResponse = serde_json::from_str(&output).unwrap();
        let mem_pool_fee = object.result.mempoolminfee;
        let update_result = set_option_value(mem_pool_fee.to_string(),name,conn);
        if let Err(err) = update_result {
            println!("node connection failure for updating fee{:?}", err);
        }
    } else {
        println!("output height {:?}",output);
        let object: BlockHeightResponse = serde_json::from_str(&output).unwrap();
        let update_result = set_option_value(object.result.to_string(),name,conn);
        if let Err(err) = update_result {
            println!("node connection failure for updating fee{:?}", err);
        } else {
            if method_type == "blockheight".to_string() {
                if node_type == "bitcoin" {
                   if(current_block_height != object.result) {
                       let _ = set_decision_maker(conn);
                   }
                }
            }
        }
    }

    Ok(())
}


fn generate_transation_hex(inputs: &Vec<UnspentVec>, to_addresses: &Vec<String>, value: Vec<f64>, pubkeys: Vec<String>, redeem: Vec<String>) -> Result<String> {
    let mut param_inputs = Vec::new();
    for input_item in inputs {
        let input_dto = CLIInputsDTO {
            txid : input_item.txid.to_string(),
            amount: input_item.amount,
            index: input_item.vout,
        };
        param_inputs.push(input_dto);
    }
    let mut param_outputs = Vec::new();

    for (i, address_input) in to_addresses.iter().enumerate() {
        if value[i]>0.0  {
            let output_dto = CLIOutputsDTO {
                address : address_input.to_string(),
                amount: value[i]
            };
            param_outputs.push(output_dto);
        }
    }

    let request_obj = TransactionRequestDTO {
        inputs: param_inputs,
        outputs: param_outputs,
        pubkeys: pubkeys,
        redeem: redeem,
        network: env!("NETWORK").to_owned()
    };


    let params = serde_json::to_string(&request_obj).unwrap();

    println!("params {:?}",params);

    let cli_response = execute_cli("-t".to_string(), params);
    if let Err(err) = cli_response {
        println!("error on transaction gemeration from cli {:?}",err);
        return Ok("".to_string());
    }
    let result = cli_response.unwrap();
    if result.status == false {
        return Ok("".to_string());
    };
    return Ok(result.result);
}

fn reduct_fee_on_values (inputs: &Vec<TxIn>,out_address: &Vec<String>, value: &Vec<f64>, node_type:&String, conn: &Connection) -> Result<Vec<f64>> {
    let mut name = "sidechain_min_fee".to_string();
    if node_type == "bitcoin" {
        name = "bitcoin_min_fee".to_string();
    }
    let sum_amount: f64 = value.clone().iter().sum();
    let min_fee = from_str::<f64>(&get_option_value(&conn,name).unwrap()).unwrap();
    let singature_size = from_str::<usize>(&env!("TRANSACTION_SIGNATURE_SIZE").to_owned()).unwrap();
    let outputs = prepare_transaction_output(out_address, value);

    let mut transaction = Transaction {
        input: inputs.clone(),
        output: outputs,
        lock_time: 0,
        version: 2,
    };

    let mut transaction_size = transaction.get_size();
  
    let members = get_members(conn, false).unwrap();
    transaction_size = transaction_size + (members.len() * singature_size);

    let per_value_percentage =  (min_fee * transaction_size as f64) / sum_amount;
    
    let mut new_value = Vec::new();
    for value_item in value {
        new_value.push(value_item - (value_item * per_value_percentage))
    }
     
    Ok(new_value)

}

fn prepare_transaction_input(address_list:&Vec<String>, value: f64, url_type: &String) -> (Vec<TxIn>, f64, Vec<UnspentVec>) {
    let confirmation_threshold = from_str::<u32>(&env!("BLOCK_CONFIRNATION_THRESHOLD").to_owned()).unwrap();
    let unspent_result = scan_transaction(address_list, url_type).unwrap();

    let current_block_height = unspent_result.result.height;
    let unspent_data = unspent_result.result.unspents.iter();


    
    let mut result_data = Vec::new();

    for detail in unspent_data {
        let datas = detail.clone().to_string();
        let detaildata: UnspentList = serde_json::from_str(&datas).unwrap();
        let height_cal = current_block_height - detaildata.height;
        if height_cal > confirmation_threshold {
            result_data.push(UnspentVec::new(
                detaildata.desc,
                detaildata.amount,
                detaildata.txid,
                detaildata.vout,
            ));
        }
    }
    result_data.sort_by(|a, b| b.partial_cmp(a).unwrap());

    let mut required_amout: f64 = 0.00;
    let mut input_tr = Vec::new();
    let mut input_unspent_data = Vec::new();

    for unspent_item in result_data {
        input_tr.push(TxIn {
            previous_output: OutPoint {
                txid: Txid::from_hex(&unspent_item.txid).unwrap(),
                vout: unspent_item.vout,
            },
            sequence: RBF,
            witness: Vec::new(),
            script_sig: Script::new(),
        });
        if value > 0.0 {
            required_amout += unspent_item.amount;
            input_unspent_data.push(unspent_item);
            if required_amout > value {
                break;
            }
        }
    }

    return (input_tr, required_amout, input_unspent_data);

}

fn prepare_transaction_output(to_address: &Vec<String>, value: &Vec<f64>) -> Vec<TxOut> {
    let mut output_tr = Vec::new();
    for (i, address_input) in to_address.iter().enumerate() {
        let balance_format = 1.0;
        let out_amount = Amount::from_btc(balance_format).unwrap();
        output_tr.push(TxOut {
            script_pubkey: Script::new(),
            value: out_amount.as_sat(),
        });
    }
    return output_tr
}

pub fn sum_of_deposit(text: String) -> Result<f64> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut stmt;
    if text != "completed" {
        stmt =
        conn.prepare("SELECT COALESCE(sum(amount), 0.00) as value FROM federation_deposits WHERE status!='Completed'")?;
    }else{
        stmt =
        conn.prepare("SELECT COALESCE(sum(amount), 0.00) as value FROM federation_deposits WHERE status='Completed'")?;
    }
    let option_list_iter =
        stmt.query_map(NO_PARAMS, |row| Ok(SumDeposit { value: row.get(0)? }))?;

    let mut option_fee = 0.00;
    for option_list in option_list_iter {
        option_fee = option_list.unwrap().value;
    }
    return Ok(format!("{:.8}", option_fee).parse::<f64>().unwrap());
}

pub fn sum_of_withdraw(text: String) -> Result<(f64)> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut stmt;
    if text != "completed" {
        stmt =
        conn.prepare("SELECT COALESCE(sum(amount), 0.00) as value FROM federation_withdraw WHERE status!='Completed'")?;
    } else {
        stmt =
        conn.prepare("SELECT COALESCE(sum(amount), 0.00) as value FROM federation_withdraw WHERE status='Completed'")?;
    }
    let withdraw_list_iter =
        stmt.query_map(NO_PARAMS, |row| Ok(SumWithdraw { value: row.get(0)? }))?;

    let mut sum_of_withdraw_amount = 0.00;
    for withdraw_data in withdraw_list_iter {
        sum_of_withdraw_amount = withdraw_data.unwrap().value;
    }
    return Ok(format!("{:.8}", sum_of_withdraw_amount)
        .parse::<f64>()
        .unwrap());
}

pub fn chart_of_deposit(start: String, end: String) -> Result<Vec<ChartDeposit>> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut stmt =
        conn.prepare("SELECT created_date, amount FROM federation_deposits WHERE strftime('%Y-%m-%d', created_date) BETWEEN ?1 AND ?2 ORDER BY created_date ASC")?;
    let deposit_list_iter = stmt.query_map(
        [&start, &end],
        |row| {
            Ok(ChartDeposit {
                xaxis: row.get(0)?,
                value: row.get(1)?,
            })
        },
    )?;

    let mut total_deposit_records = Vec::new();
    for deposit_data in deposit_list_iter {
        total_deposit_records.push(deposit_data.unwrap());
    }
    return Ok(total_deposit_records);
}

pub fn chart_of_withdraw(start: String, end: String) -> Result<Vec<ChartWithdraw>> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut stmt =
        conn.prepare("SELECT created_date, amount FROM federation_withdraw WHERE strftime('%Y-%m-%d', created_date) BETWEEN ?1 AND ?2 ORDER BY created_date ASC")?;
    let withdraw_list_iter = stmt.query_map([&start, &end], |row| {
        Ok(ChartWithdraw {
            xaxis: row.get(0)?,
            value: row.get(1)?,
        })
    })?;
    
    let mut total_withdraw_records = Vec::new();
    for withdraw_data in withdraw_list_iter {
        total_withdraw_records.push(withdraw_data.unwrap());
    }
    return Ok(total_withdraw_records);
}

pub fn list_of_address() -> Result<Vec<ListAddress>> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut stmt =
        conn.prepare("SELECT mutlisig_address, indexer, created_date FROM federation_address")?;
    let address_list_iter = stmt.query_map(NO_PARAMS, |row| {
        Ok(ListAddress {
            mutlisig_address: row.get(0)?,
            indexer: row.get(1)?,
            created_date: row.get(2)?,
        })
    })?;

    let mut total_address_records = Vec::new();
    for address_data in address_list_iter {
        total_address_records.push(address_data.unwrap());
    }
    return Ok(total_address_records);
}

pub fn list_of_history(page_size: i64, text: String, types: i64) -> Result<Vec<ListHistory>> {
    let start_from = (page_size - 1) * 10;
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut total_history_records = Vec::new();
    if text.len() > 0 {
        if types != 0 {
            let mut stmt = conn.prepare("SELECT address, tx_id, amount, fee, type, created_at FROM federation_history WHERE (address LIKE :input_text OR tx_id LIKE :input_text OR amount LIKE :input_text OR fee LIKE :input_text OR created_at LIKE :input_text) AND type=:astype ORDER BY created_at DESC LIMIT :offset,:limit")?;

            let percentage_symbol = "%".to_string();
            let search_text = percentage_symbol.clone()+&text+&percentage_symbol.clone();

            let history_list_iter = stmt.query_map(
                &[
                    (":input_text", search_text.to_string().as_str()),
                    (":offset", start_from.to_string().as_str()),
                    (":limit", "10".to_string().as_str()),
                    (":astype", types.to_string().as_str()),
                ],
                |row| {
                    Ok(ListHistory {
                        address: row.get(0)?,
                        tx_id: row.get(1)?,
                        amount: row.get(2)?,
                        fee: row.get(3)?,
                        types: row.get(4)?,
                        created_date: row.get(5)?,
                    })
                },
            )?;

            for history_data in history_list_iter {
                total_history_records.push(history_data.unwrap());
            }
            return Ok(total_history_records);
        } else {
            let mut stmt = conn.prepare("SELECT address, tx_id, amount, fee, type, created_at FROM federation_history WHERE (address LIKE :input_text OR tx_id LIKE :input_text OR amount LIKE :input_text OR fee LIKE :input_text OR created_at LIKE :input_text)  ORDER BY created_at DESC LIMIT :offset,:limit")?;
            let percentage_symbol = "%".to_string();
            let search_text = percentage_symbol.clone()+&text+&percentage_symbol.clone();
            let history_list_iter = stmt.query_map(
                &[
                    (":input_text", search_text.to_string().as_str()),
                    (":offset", start_from.to_string().as_str()),
                    (":limit", "10".to_string().as_str()),
                ],
                |row| {
                    Ok(ListHistory {
                        address: row.get(0)?,
                        tx_id: row.get(1)?,
                        amount: row.get(2)?,
                        fee: row.get(3)?,
                        types: row.get(4)?,
                        created_date: row.get(5)?,
                    })
                },
            )?;

            for history_data in history_list_iter {
                total_history_records.push(history_data.unwrap());
            }
            return Ok(total_history_records);
        }
    } else {
        if types != 0 {
            let mut stmt = conn.prepare("SELECT address, tx_id, amount, fee, type, created_at FROM federation_history WHERE type=?3 ORDER BY created_at DESC LIMIT ?1,?2")?;
            let history_list_iter = stmt.query_map(
                [
                    start_from.to_string().as_str(),
                    "10".to_string().as_str(),
                    types.to_string().as_str(),
                ],
                |row| {
                    Ok(ListHistory {
                        address: row.get(0)?,
                        tx_id: row.get(1)?,
                        amount: row.get(2)?,
                        fee: row.get(3)?,
                        types: row.get(4)?,
                        created_date: row.get(5)?,
                    })
                },
            )?;
            let mut total_history_records = Vec::new();
            for history_data in history_list_iter {
                total_history_records.push(history_data.unwrap());
            }
            return Ok(total_history_records);
        } else {
            let mut stmt = conn.prepare("SELECT address, tx_id, amount, fee, type, created_at FROM federation_history  ORDER BY created_at DESC LIMIT ?1,?2")?;
            let history_list_iter = stmt.query_map(
                [start_from.to_string().as_str(), "10".to_string().as_str()],
                |row| {
                    Ok(ListHistory {
                        address: row.get(0)?,
                        tx_id: row.get(1)?,
                        amount: row.get(2)?,
                        fee: row.get(3)?,
                        types: row.get(4)?,
                        created_date: row.get(5)?,
                    })
                },
            )?;
            let mut total_history_records = Vec::new();
            for history_data in history_list_iter {
                total_history_records.push(history_data.unwrap());
            }
            return Ok(total_history_records);
        }
    }
}

pub fn total_history_count(text: String, types: i64) -> Result<i64> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut total_records = 0;
    if text.len() > 0 {
        if types != 0 {
            let percentage_symbol = "%".to_string();
            let search_text = percentage_symbol.clone()+&text+&percentage_symbol.clone();

            let mut stmt = conn.prepare("SELECT COALESCE(count(*), 0) as value FROM federation_history WHERE (address LIKE :input_text OR tx_id LIKE :input_text OR amount LIKE :input_text OR fee LIKE :input_text OR created_at LIKE :input_text) AND type=:astype")?;
            let history_list_iter = stmt.query_map(
                &[
                    (":input_text", search_text.to_string().as_str()),
                    (":astype", types.to_string().as_str()),
                ],
                |row| Ok(TotalHistory { total: row.get(0)? }),
            )?;

            for history_data in history_list_iter {
                total_records = history_data.unwrap().total;
            }
            return Ok(total_records);
        } else {
            let percentage_symbol = "%".to_string();
            let search_text = percentage_symbol.clone()+&text+&percentage_symbol.clone();

            let mut stmt = conn.prepare("SELECT COALESCE(count(*), 0) as value FROM federation_history WHERE (address LIKE :input_text OR tx_id LIKE :input_text OR amount LIKE :input_text OR fee LIKE :input_text OR created_at LIKE :input_text)")?;
            let history_list_iter = stmt
                .query_map(&[(":input_text", search_text.to_string().as_str())], |row| {
                    Ok(TotalHistory { total: row.get(0)? })
                })?;

            for history_data in history_list_iter {
                total_records = history_data.unwrap().total;
            }
            return Ok(total_records);
        }
    } else {
        if types != 0 {
            let mut stmt = conn.prepare(
                "SELECT COALESCE(count(*), 0) as value FROM federation_history WHERE type=?1",
            )?;
            let history_list_iter = stmt.query_map([types.to_string().as_str()], |row| {
                Ok(TotalHistory { total: row.get(0)? })
            })?;

            for history_data in history_list_iter {
                total_records = history_data.unwrap().total;
            }
            return Ok(total_records);
        } else {
            let mut stmt =
                conn.prepare("SELECT COALESCE(count(*), 0) as value FROM federation_history")?;
            let history_list_iter =
                stmt.query_map([], |row| Ok(TotalHistory { total: row.get(0)? }))?;

            for history_data in history_list_iter {
                total_records = history_data.unwrap().total;
            }
            return Ok(total_records);
        }
    }
}

pub fn fee_listing(text: String) -> Result<Vec<FeeData>> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    let mut stmt = conn.prepare("SELECT value from federation_options WHERE name=:name")?;
    let options_iter = stmt.query_map(&[(":name", text.to_string().as_str())], |row| {
        Ok(FeeData { value: row.get(0)? })
    })?;
    let mut options_records = Vec::new();
    for options_data in options_iter {
        options_records.push(options_data.unwrap());
    }
    return Ok(options_records);
}

pub fn balance_listing(text: String) -> Result<f64> {
    let conn = Connection::open(env!("DATABASE").to_owned())?;
    println!("input node {:?}", text);
    let master_details = get_master_details(&conn);
    let master_address = master_details.1;
    println!("master info {:?}", master_address);
    let prepare_input = vec![master_address];
    let unspent_result = scan_transaction(&prepare_input, &text).unwrap(); 
    let total_amount = unspent_result.result.total_amount;
    return Ok(total_amount);
}
