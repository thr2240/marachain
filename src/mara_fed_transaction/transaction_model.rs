
use serde::{Serialize, Deserialize};
use super::transaction_utils::{
    sum_of_deposit, sum_of_withdraw, ChartDeposit, ChartWithdraw, ListHistory, ListAddress, FeeData
};

#[derive(Serialize, Deserialize)]
pub struct SignModel {
    pub hex: String,
}

pub struct SumDepositModel {
    amount: f64,
}
impl SumDepositModel {
    pub fn new(text:String) -> SumDepositModel {
        return SumDepositModel {
            amount: sum_of_deposit(text).unwrap(),
        };
    }
    pub fn sum_of_deposit(&self,text:String) -> f64 {
        return sum_of_deposit(text).unwrap();
    }
}

pub struct SumWithdrawModel {
    amount: f64,
}
impl SumWithdrawModel {
    pub fn new(text:String) -> SumWithdrawModel {
        return SumWithdrawModel {
            amount: sum_of_withdraw(text).unwrap(),
        };
    }
    pub fn sum_of_withdraw(&self,text:String) -> f64 {
        return sum_of_withdraw(text).unwrap();
    }
}

#[derive(Serialize)]
pub struct ChartDepositModel {
    pub data: Vec<ChartDeposit>,
}

#[derive(Serialize)]
pub struct ChartWithdrawModel {
    pub data: Vec<ChartWithdraw>,
}

#[derive(Serialize)]
pub struct ListHistoryModel {
    pub data: Vec<ListHistory>,
    pub total: i64
}


#[derive(Serialize)]
pub struct ListAddressModel {
    pub data: Vec<ListAddress>,
}


#[derive(Serialize)]
pub struct FeeDataModel {
    pub data: Vec<FeeData>,
}

#[derive(Serialize)]
pub struct BalanceDataModel {
    pub data: f64,
}
