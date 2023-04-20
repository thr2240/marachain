use serde::Deserialize;

#[derive(Deserialize,Debug)]
pub struct SignDTO {
   pub hex: String,
   pub index: String,
   pub account_type: String
}