use std::collections::HashMap;
use near_sdk::collections::{ UnorderedMap, LookupMap};
use near_sdk::{AccountId, Balance, env, near_bindgen, BorshStorageKey };
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    AccountTokens {account_id: AccountId},
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Account {
    pub near_amount: Balance,
    pub tokens : UnorderedMap<AccountId, Balance>
}

impl Account {
    pub fn new(account_id: &AccountId) -> Self {
        Account {
            near_amount: 0,
            tokens: UnorderedMap::new(StorageKey::AccountTokens {
                account_id: account_id.clone(),
            }),
        }
    }

    pub fn get_balance (&self, token_id: &AccountId) -> Option<Balance> {
        if let Some(token_balance) = self.tokens.get(token_id) {
            Some(token_balance)
        } else {
            None
        }
    }

    pub fn register (&mut self, token_ids: &Vec<ValidAccountId>) {
        for token_id in token_ids {
            let t = token_id.as_ref();
            if self.get_balance(t).is_none() {
                self.tokens.insert(t, &0);
            }
        }
    }
}
#[near_bindgen]
pub struct Contract {
    owner_id: AccountId,
    accounts: LookupMap<AccountId, Account>,
}

#[near_bindgen]

impl Contract {
    #[payable]
    pub fn register_tokens(&mut self, token_ids: Vec<ValidAccountId>) {
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        account.register(&token_ids);

    }
}

impl Contract {
    pub fn internal_save_account(&mut self, account_id: &AccountId, account: Account) {
        
    }

    pub fn internal_get_account(&self, account_id: &AccountId) -> Option<Account> {
        self.accounts
            .get(account_id)
    }

    pub fn internal_unwrap_account(&self, account_id: &AccountId) -> Account {
        self.internal_get_account(account_id)
            .expect("ACCOUNT NOT REGISTERED")
    }
}