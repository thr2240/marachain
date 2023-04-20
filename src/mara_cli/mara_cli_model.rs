

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CLIResponseModel {
    pub status: bool,
    pub result: String
}
