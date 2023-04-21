use serde::{Deserialize, Serialize};

use super::member_utils::FedMember;


#[derive(Debug, Serialize, Deserialize)]
pub struct MemberListModel {
    pub data: Vec<FedMember>,
}
