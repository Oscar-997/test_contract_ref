use near_sdk::{serde::{Serialize, Deserialize}, AccountId, json_types::U128, near_bindgen};

use crate::{pool::Pool, utils::SwapVolume};
use crate::*;

pub const ERR85_NO_POOL: &str = "E85: invalid pool id";



#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct PoolInfo {
    /// Pool kind.
    pub pool_kind: String,
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// How much NEAR this contract has.
    pub amounts: Vec<U128>,
    /// Fee charged for swap.
    pub total_fee: u32,
    /// Total number of shares.
    pub shares_total_supply: U128,
    pub amp: u64,
}

impl From<Pool> for PoolInfo {
    fn from(pool: Pool) -> Self {
        let pool_kind = pool.kind();
        match pool {
            Pool::SimplePool(pool) => Self {
                pool_kind,
                amp: 0,
                token_account_ids: pool.token_account_ids,
                amounts: pool.amounts.into_iter().map(|a| U128(a)).collect(),
                total_fee: pool.total_fee,
                shares_total_supply: U128(pool.shares_total_supply),
            }
        }
    }
}

#[near_bindgen]
impl Contract {
    /// Returns list of pools of given length from given start index
    pub fn get_pools(&self, from_index: u64, limit: u64) -> Vec<PoolInfo> {
        (from_index..std::cmp::min(from_index + limit, self.pools.len()))
            .map(|index| self.get_pool(index))
            .collect()
    }

    /// Returns information about specified pool.
    pub fn get_pool(&self, pool_id: u64) -> PoolInfo {
        self.pools.get(pool_id).expect("ERR_NO_POOL").into()
    }

    /// Return volumes of the given pool.
    pub fn get_pool_volumes(&self, pool_id: u64) -> Vec<SwapVolume> {
        self.pools.get(pool_id).expect("ERR_NO_POOL").get_volumes()
    }

    /// Returns number of shares given account has in given pool.
    pub fn get_pool_shares(&self, pool_id: u64, account_id: ValidAccountId) -> U128 {
        self.pools
            .get(pool_id)
            .expect("ERR_NO_POOL")
            .share_balances(account_id.as_ref())
            .into()
    }

    /// Returns total number of shares in the given pool.
    pub fn get_pool_total_shares(&self, pool_id: u64) -> U128 {
        self.pools
            .get(pool_id)
            .expect(ERR85_NO_POOL)
            .share_total_balance()
            .into()
    }

    /// Returns number of pools 
    pub fn get_number_of_pools(&self) -> u64 {
        self.pools.len()
    }
}