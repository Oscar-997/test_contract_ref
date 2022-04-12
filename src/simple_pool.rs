use std::cmp::min;

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{env, AccountId, Balance};

use crate::StorageKey;
use crate::utils::{ SwapVolume, FEE_DIVISOR, U256, add_to_collection, integer_sqrt };

const NUM_TOKENS: usize = 2;
const ERR14_LP_ALREADY_REGISTERED: &str = "E14: LP already registered";
const ERR13_LP_NOT_REGISTERED: &str = "E13: LP not registered";
const ERR31_ZERO_AMOUNT: &str = "E31: adding zero amount";
const ERR32_ZERO_SHARES: &str = "E32: minting zero shares";
pub const INIT_SHARES_SUPPLY: u128 = 1_000_000_000_000_000_000_000_000;



#[derive(BorshSerialize, BorshDeserialize)]
pub struct SimplePool {
    /// List of tokens in the pool.
    pub token_account_ids: Vec<AccountId>,
    /// How much NEAR this contract has.
    pub amounts: Vec<Balance>,
    ///Volumes accumulated by this pool.
    pub volumes: Vec<SwapVolume>,
    /// Fee charged for swap (gets divided by FEE_DEIVISOR).
    pub total_fee: u32,
    /// Obsolete, reserve to simplify upgrade.
    pub exchange_fee: u32,
    // Obsolete, reserve to simplify upgrade.
    pub referral_fee: u32,
    /// Shares of the pool by liquidity providers.
    pub shares: LookupMap<AccountId, Balance>,
    /// Total number of shares.
    pub shares_total_supply: Balance,
}

impl SimplePool {
    pub fn new (
        id: u32,
       token_account_ids: Vec<ValidAccountId>,
       total_fee: u32,
       exchange_fee: u32,
       referral_fee: u32, 
    ) -> Self {
       assert!(
           total_fee < FEE_DIVISOR,
           "ERR_FEE_TOO_LARGE"
       );

       // [AUDIT_10]
       assert_eq!(token_account_ids.len(), NUM_TOKENS, "ERR_SHOULD_HAVE_2_TOKENS");
       Self {
           token_account_ids: token_account_ids.iter().map(|a| a.clone().into()).collect(),
           amounts: vec![0u128; token_account_ids.len()],
           volumes: vec![SwapVolume::default(); token_account_ids.len()],
           total_fee,
           exchange_fee,
           referral_fee,
           // [AUDIT_11]
           shares: LookupMap::new(StorageKey::Shares {
               pool_id: id,
           }),
           shares_total_supply: 0,
       }
    }

    /// Register given account with 0 balance in shares.
    /// Storage payment should be checked by caller.
    pub fn share_register(&mut self, account_id: &AccountId) {
        if self.shares.contains_key(account_id) {
            env::panic(ERR14_LP_ALREADY_REGISTERED.as_bytes());
        }
        self.shares.insert(account_id, &0);
    }

    /// Transfer shares from predecessor to receiver.
    pub fn share_transfer(&mut self, sender_id: &AccountId, receiver_id: &AccountId, amount: u128) {
        let balance = self.shares.get(&sender_id).expect("ERR_NO_SHARES");
        if let Some(new_balance) = balance.checked_sub(amount) {
            self.shares.insert(&sender_id, &new_balance);
        } else {
            env::panic(b"ERR_NOT_ENOUGH_SHARES")
        }
        let balance_out = self
            .shares
            .get(&receiver_id)
            .expect(ERR13_LP_NOT_REGISTERED);
        self.shares.insert(&receiver_id, &(balance_out + amount));
    }

    /// Returns balance of shares for given user.
    pub fn share_balance_of(&self, account_id: &AccountId) -> Balance {
        self.shares.get(account_id).unwrap_or_default()
    }

    /// Returns total number of shares in this pool.
    pub fn share_total_balance(&self) -> Balance {
        self.shares_total_supply
    }

    /// Returns list of tokens in this pool.
    pub fn tokens(&self) -> &[AccountId] {
        &self.token_account_ids
    }

    /// Adds the amounts of tokens to liquidity pool and returns number of shares that this user receives.
    pub fn add_liquidity(&mut self, sender_id: &AccountId, amounts: &mut Vec<Balance>) -> Balance {
        assert_eq!(
            amounts.len(),
            self.token_account_ids.len(),
            "ERR_WRONG_TOKEN_COUNT"
        );
        let shares = if self.shares_total_supply > 0 {
            let mut fair_supply = U256::max_value();
            for i in 0..self.token_account_ids.len() {
                assert!(amounts[i] > 0, "{}", ERR31_ZERO_AMOUNT );
                fair_supply = min(
                    fair_supply,
                    U256::from(amounts[i]) * U256::from(self.shares_total_supply) / self.amounts[i],
                );
            }
            for i in 0..self.token_account_ids.len() {
                let amount = (U256::from(self.amounts[i]) * fair_supply
                    / U256::from(self.shares_total_supply))
                .as_u128();
                assert!(amount > 0, "{}", ERR31_ZERO_AMOUNT);
                self.amounts[i] += amount;
                amounts[i] = amount;
            }
            fair_supply.as_u128()
        } else {
            for i in 0..self.token_account_ids.len() {
                self.amounts[i] += amounts[i];
            }
            INIT_SHARES_SUPPLY
            
        };
        self.mint_shares(&sender_id, shares);
        assert!(shares > 0, "{}", ERR32_ZERO_SHARES);
        env::log(
            format!(
                "liquidity added {:?}, minted {} shares",
                amounts
                    .iter()
                    .zip(self.token_account_ids.iter())
                    .map(|(amount, token_id)| format!("{} {}", amount, token_id))
                    .collect::<Vec<String>>(),
                shares
            )
            .as_bytes(),
        );
        shares
    }

    /// Mint new shares for given user.
    fn mint_shares(&mut self, account_id: &AccountId, shares: Balance) {
        if shares == 0 {
            return;
        }
        self.shares_total_supply += shares;
        add_to_collection(&mut self.shares, &account_id, shares);
    }

    /// Removes given number of shares form the pool and returns amounts to the parent.
    pub fn remove_liquidity(
        &mut self, 
        sender_id: &AccountId,
        shares: Balance,
        min_amounts: Vec<Balance>,
    ) -> Vec<Balance> {
        assert_eq!(
            min_amounts.len(),
            self.token_account_ids.len(),
            "ERR_WRONG_TOKEN_COUNT"
        );
        let prev_shares_amount = self.shares.get(&sender_id).expect("ERR_NO_SHARES");
        assert!(prev_shares_amount >= shares, "ERR_NOT_ENOUGH_SHARES");
        let mut result = vec![];
        for i in 0..self.token_account_ids.len() {
            let amount = (U256::from(self.amounts[i]) * U256::from(shares)
                / U256::from(self.shares_total_supply))
            .as_u128();
            assert!(amount >= min_amounts[i], "ERR_MIN_AMOUNT");
            self.amounts[i] -= amount;
            result.push(amount);
        }
        if prev_shares_amount == shares {
            self.shares.insert(&sender_id, &0);
        } else {
            self.shares
                .insert(&sender_id, &(prev_shares_amount - shares));
        }
        env::log(
            format!(
                "{} shares of liquidity removed: receive back {:?}", 
                shares,
                result
                    .iter()
                    .zip(self.token_account_ids.iter())
                    .map(|(amount, token_id)| format!("{} {}", amount, token_id))
                    .collect::<Vec<String>>(),
            )
            .as_bytes(),
        );
        self.shares_total_supply -= shares;
        result
    }

    /// Returns token index for given pool.
    fn token_index(&self, token_id: &AccountId) -> usize {
        self.token_account_ids
            .iter()
            .position(|id| id == token_id)
            .expect("ERR_MISSING_TOKEN")
    }

    /// Returns number of tokens in outcome, given amount.
    /// Tokens are provided as indexes into token list for given pool.
    fn internal_get_return(
        &self,
        token_in: usize,
        amount_in: Balance,
        token_out: usize
    ) -> Balance {
        let in_balance = U256::from(self.amounts[token_in]);
        let out_balance = U256::from(self.amounts[token_out]);
        assert!(
            in_balance > U256::zero()
                && out_balance > U256::zero()
                && token_in != token_out
                && amount_in > 0,
            "ERR_INVALID"
        );
        let amount_with_fee = U256::from(amount_in) * U256::from(FEE_DIVISOR - self.total_fee);
        (amount_with_fee * out_balance / (U256::from(FEE_DIVISOR) * in_balance + amount_with_fee))
            .as_u128()
    }

    /// Returns how much token you will receive if swap `token_amount_in` of `token_in` for `token_out`.
    pub fn get_return(
        &self,
        token_in: &AccountId,
        amount_in: Balance,
        token_out: &AccountId,
    ) -> Balance {
        self.internal_get_return(
            self.token_index(token_in),
            amount_in,
            self.token_index(token_out)
        )
    }

    /// Returns given pool's total fee.
    pub fn get_fee(&self)-> u32 {
        self.total_fee
    }

    /// Returns volumes of the given pool.
    pub fn get_volumes(&self) -> Vec<SwapVolume> {
        self.volumes.clone()
    }
}
