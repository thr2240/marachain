use super::{
    scanner_model::DepositListModel,
    scanner_utils::{
        get_deposits_count, get_deposits_list, get_withdraws_count, get_withdraws_list,
    },
};
use actix_web::{web::Json, Responder, Result};

pub fn list_deposits(
    search_term: String,
    limit: usize,
    offset: usize,
) -> Result<Json<DepositListModel>> {
    let result = get_deposits_list(search_term.clone(), limit, offset).unwrap();
    let total_docs = get_deposits_count(search_term).unwrap();
    if result.len() == 0 {
        let obj = DepositListModel {
            status: false,
            message: "Something went wrong".to_string(),
            totalDocs: 0,
            result: result,
        };
        return Ok(Json(obj));
    }
    let obj = DepositListModel {
        status: true,
        message: "Deposit list retreived successfully".to_string(),
        totalDocs: total_docs,
        result: result,
    };
    Ok(Json(obj))
}

pub fn list_withdraws(search_term: String, limit: usize, offset: usize) -> Result<impl Responder> {
    let result = get_withdraws_list(search_term.clone(), limit, offset).unwrap();
    let total_docs = get_withdraws_count(search_term).unwrap();
    if result.len() == 0 {
        let obj = DepositListModel {
            status: false,
            message: "Something went wrong".to_string(),
            totalDocs: 0,
            result: result,
        };
        return Ok(Json(obj));
    }
    let obj = DepositListModel {
        status: true,
        message: "Deposit list retreived successfully".to_string(),
        totalDocs: total_docs,
        result: result,
    };
    Ok(Json(obj))
}

#[cfg(test)]
mod tests {
    use super::super::scanner_model::DepositList;
    use super::*;
    use futures::future::ready;
    pub fn list_deposits_with_deps(
        search_term: String,
        limit: usize,
        offset: usize,
        get_deposits_list_fn: &dyn Fn(String, usize, usize) -> Result<Vec<DepositList>>,
        get_deposits_count_fn: &dyn Fn(String) -> Result<usize>,
    ) -> Result<Json<DepositListModel>> {
        let result = get_deposits_list_fn(search_term.clone(), limit, offset)?;
        let total_docs = get_deposits_count_fn(search_term.clone())?;
        list_deposits(search_term, limit, offset)
    }

    fn mock_get_deposits_list(
        _search_term: String,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<DepositList>> {
        Ok(vec![DepositList {
            id: 1,
            sender_address: "address1".to_string(),
            receiver_address: "address2".to_string(),
            txid: "txid1".to_string(),
            amount: 0.1,
            confirmations: 1,
            status: "pending".to_string(),
            date: "2023-01-01".to_string(),
        }])
    }

    fn mock_get_deposits_count(_search_term: String) -> Result<usize> {
        Ok(1)
    }

    #[actix_rt::test]
    async fn test_list_deposits() {
        let search_term = "".to_string();
        let limit = 10;
        let offset = 0;

        let result = ready(list_deposits_with_deps(
            search_term,
            limit,
            offset,
            &mock_get_deposits_list,
            &mock_get_deposits_count,
        ))
        .await;
        let response = result.unwrap();
        let deposit_list_model = response.into_inner();
        assert_eq!(deposit_list_model.status, true);
        assert_eq!(deposit_list_model.totalDocs, 1);
    }

    fn list_withdraws_with_deps(
        search_term: String,
        limit: usize,
        offset: usize,
        get_withdraws_list_fn: &dyn Fn(String, usize, usize) -> Result<Vec<DepositList>>,
        get_withdraws_count_fn: &dyn Fn(String) -> Result<usize>,
    ) -> Result<Json<DepositListModel>> {
        let result = get_withdraws_list_fn(search_term.clone(), limit, offset)?;
        let total_docs = get_withdraws_count_fn(search_term.clone())?;
        list_deposits(search_term, limit, offset)
    }
    // Mock functions
    fn mock_get_withdraws_list(
        _search_term: String,
        _limit: usize,
        _offset: usize,
    ) -> Result<Vec<DepositList>> {
        Ok(vec![DepositList {
            id: 1,
            sender_address: "sender_address".to_string(),
            receiver_address: "receiver_address".to_string(),
            txid: "txid".to_string(),
            amount: 100.0,
            confirmations: 6,
            status: "status".to_string(),
            date: "2023-01-01".to_string(),
        }])
    }

    fn mock_get_withdraws_count(_search_term: String) -> Result<usize> {
        Ok(1)
    }

    // Unit test
    #[actix_rt::test]
    async fn test_list_withdraws() {
        let search_term = "".to_string();
        let limit = 10;
        let offset = 0;

        // Update the list_withdraws function signature
        let result = ready(list_withdraws_with_deps(
            search_term,
            limit,
            offset,
            &mock_get_withdraws_list,
            &mock_get_withdraws_count,
        ))
        .await;

        let response = result.unwrap();
        let withdraw_list_model = response.into_inner();
        assert_eq!(withdraw_list_model.status, true);
        assert_eq!(withdraw_list_model.totalDocs, 1);
    }
}
