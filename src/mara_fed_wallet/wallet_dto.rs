use serde::Deserialize;

#[derive(Deserialize,Debug)]
pub struct AccountNumberDTO {
   pub account_number: String
}