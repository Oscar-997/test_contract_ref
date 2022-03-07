use near_sdk::collections::{ UnorderedMap, LookupMap};
use near_sdk::{AccountId, Balance, env, near_bindgen, BorshStorageKey, StorageUsage };
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};

mod storage_impl;


pub const ERR11_INSUFFICIENT_STORAGE: &str = "E11: insufficient $NEAR storage deposit";
pub const ERR24_NON_ZERO_TOKEN_BALANCE: &str = "E24: non-zero token balance";


const U128_STORAGE: StorageUsage = 16;
const U64_STORAGE: StorageUsage = 8;
const U32_STORAGE: StorageUsage = 4;
/// max length of account id is 64 bytes. We charge per byte.
const ACC_ID_STORAGE: StorageUsage = 64;
/// As a key, 4 bytes length would be added to the head
const ACC_ID_AS_KEY_STORAGE: StorageUsage = ACC_ID_STORAGE + 4;
const KEY_PREFIX_ACC: StorageUsage = 64;
/// As a near_sdk::collection key, 1 byte for prefiex
const ACC_ID_AS_CLT_KEY_STORAGE: StorageUsage = ACC_ID_AS_KEY_STORAGE + 1;

// ACC_ID: the Contract accounts map key length
// + VAccount enum: 1 byte
// + U128_STORAGE: near_amount storage
// + U32_STORAGE: legacy_tokens HashMap length
// + U32_STORAGE: tokens HashMap length
// + U64_STORAGE: storage_used
pub const INIT_ACCOUNT_STORAGE: StorageUsage =
    ACC_ID_AS_CLT_KEY_STORAGE + 1 + U128_STORAGE + U32_STORAGE + U32_STORAGE + U64_STORAGE;


#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Accounts,
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

    pub fn unregister (&mut self, token_id: &AccountId) {
        let amount = self.tokens.remove(token_id).unwrap_or_default();
        assert_eq!(amount, 0, "{}", ERR24_NON_ZERO_TOKEN_BALANCE)
    }

    // [AUDIT_01]
    /// Returns amount of $NEAR necessary to cover storage used by this data structure.
    pub fn storage_usage(&self) -> Balance {
        (INIT_ACCOUNT_STORAGE + 
            self.tokens.len() as u64 * (KEY_PREFIX_ACC + ACC_ID_AS_KEY_STORAGE + U128_STORAGE)
        ) as u128
            * env::storage_byte_cost()
    }

    /// Asserts there is sufficient amount of $NEAR to cover storage usage.
    pub fn assert_storage_usage(&self) {
        assert!(
            self.storage_usage() <= self.near_amount,
            "{}",
            ERR11_INSUFFICIENT_STORAGE
        );
    }
}


#[near_bindgen]
pub struct Contract {
    owner_id: AccountId,
    accounts: LookupMap<AccountId, Account>,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(owner_id: ValidAccountId) -> Self {
        Self {
            owner_id: owner_id.as_ref().clone(),
            accounts: LookupMap::new(StorageKey::Accounts)
        }
    }

    #[payable]
    pub fn register_tokens(&mut self, token_ids: Vec<ValidAccountId>) {
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        account.register(&token_ids);
        self.internal_save_account(&sender_id, account);
    }

    #[payable]
    pub fn unregister_tokens(&mut self, token_ids: Vec<ValidAccountId>) {
        let sender_id = env::predecessor_account_id();
        let mut account = self.internal_unwrap_account(&sender_id);
        for token_id in token_ids {
            account.unregister(token_id.as_ref())
        }
        self.internal_save_account(&sender_id, account)
    }
}

impl Contract {
    pub(crate) fn internal_save_account(&mut self, account_id: &AccountId, account: Account) {
        account.assert_storage_usage();
        self.accounts.insert(&account_id, &account.into());
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