use std::process::{Command, Stdio};
use actix_web::{
    Result
};
use crate::mara_cli::mara_cli_model::CLIResponseModel;

pub fn execute_cli(method: String, params: String) -> Result<CLIResponseModel> {
    let output = Command::new(env!("CLI_LOCATION").to_owned())
    .args([method,params])
    .stdout(Stdio::piped())
    .output()
    .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    let response = serde_json::from_str::<CLIResponseModel>(&stdout);
    Ok(response.unwrap())
}