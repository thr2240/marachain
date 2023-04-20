use serde::Serialize;

use super::member_utils::FedMember;

#[derive(Serialize)]
pub struct MemberListModel {
    pub data: Vec<FedMember>
}
