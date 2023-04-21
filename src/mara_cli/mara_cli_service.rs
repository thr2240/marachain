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

#[cfg(test)]
mod tests {
    use super::execute_cli;
    use crate::mara_cli::mara_cli_model::CLIResponseModel;
    use actix_web::Result;

    #[test]
    fn test_execute_cli() -> Result<()> {

        let method = "method_for_test".to_string();
        let params = "params_for_test".to_string();

        let response: CLIResponseModel = execute_cli(method, params)?;

        assert_eq!(response.status, true, "The response field does not match the expected value");

        Ok(())
    }
}
