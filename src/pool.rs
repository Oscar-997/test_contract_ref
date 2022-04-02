use near_sdk::{AccountId, Balance};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};

use crate::simple_pool::SimplePool;
use crate::utils::SwapVolume;

#[derive(BorshSerialize, BorshDeserialize)]
pub enum Pool {
    SimplePool(SimplePool),
}

impl Pool {
    /// Returns pool kind.
    pub fn kind(&self) -> String {
        match self {
            Pool::SimplePool(_) => "SIMPLE_POOL".to_string()
        }
    }


    pub fn share_register(&mut self, account_id: &AccountId) {
        match self {
            Pool::SimplePool(pool) => pool.share_register(account_id)
        }
    }

    pub fn add_liquidity(
        &mut self, 
        sender_id: &AccountId,
        amounts: &mut Vec<Balance>,
    ) -> Balance {
        match self {
            Pool::SimplePool(pool) => pool.add_liquidity(sender_id, amounts)
        }
    }

    pub fn tokens(&self) -> &[AccountId] {
        match self {
            Pool::SimplePool(pool) => pool.tokens()
        }
    }

    pub fn remove_liquidity(
        &mut self,
        sender_id: &AccountId,
        shares: Balance,
        min_amounts: Vec<Balance>,
    ) -> Vec<Balance> {
        match self {
            Pool::SimplePool(pool) => pool.remove_liquidity(sender_id, shares, min_amounts)
        }
    }

    /// Returns volumes of the given pool.
    pub fn get_volumes(&self) -> Vec<SwapVolume> {
        match self {
            Pool::SimplePool(pool) => pool.get_volumes(),
        }
    }

    pub fn share_balances(&self, account_id: &AccountId) -> Balance {
        match self {
            Pool::SimplePool(pool) => pool.share_balance_of(account_id)
        }
    }
}


