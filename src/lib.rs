use near_sdk::collections::{ UnorderedMap, LookupMap};
use near_sdk::{AccountId, Balance, env, near_bindgen,
     BorshStorageKey, StorageUsage, log, Promise, Gas
     };
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};

mod storage_impl;

pub const GAS_FOR_FT_TRANSFER: Gas = 20_000_000_000_000;
pub const GAS_FOR_RESOLVE_TRANSFER: Gas = 20_000_000_000_000;

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
// + U32_STORAGE: tokens HashMap length
// + U64_STORAGE: storage_used
pub const INIT_ACCOUNT_STORAGE: StorageUsage =
    ACC_ID_AS_CLT_KEY_STORAGE + 1 + U128_STORAGE + U32_STORAGE + U64_STORAGE;


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

impl Default for Contract {
    fn default() -> Self {
        env::panic(b"Must initilize contract before using");
        Self {
            owner_id: env::predecessor_account_id(),
            accounts: LookupMap::new(StorageKey::Accounts),
        }
    }
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

    /// Returns minimal account deposit storage usage possible.
    pub fn min_storage_usage() -> Balance {
        INIT_ACCOUNT_STORAGE as Balance * env::storage_byte_cost()
    }

    // 
    pub fn storage_available(&self) -> Balance {
        let locked = self.storage_usage();
        if self.near_amount > locked {
            self.near_amount - locked
        } else {
            0
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
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
    pub fn internal_save_account(&mut self, account_id: &AccountId, account: Account) {
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

    pub fn internal_register_account(&mut self, account_id: &AccountId, amount: Balance) {
        let mut account = self.internal_unwrap_or_default_account(&account_id);
        account.near_amount += amount;
        self.internal_save_account(&account_id, account);
    }

    pub fn internal_unwrap_or_default_account (&self, account_id: &AccountId) -> Account {
        self.internal_get_account(account_id)
            .unwrap_or_else(|| Account::new(account_id))
    }

    pub fn internal_storage_withdraw(&mut self, account_id: &AccountId, amount: Balance) -> u128 {
        let mut account = self.internal_unwrap_account(&account_id);
        let available = account.storage_available();
        assert!(available > 0, "Not storage withdraw");
        let mut withdraw_amount = amount;
        if amount == 0 {
            withdraw_amount = available;
        }
        assert!(withdraw_amount <= available, "storage withdraw to much");
        account.near_amount -= withdraw_amount;
        self.internal_save_account(account_id, account);
        withdraw_amount
    }

    pub fn internal_save_information_to_contract(
        &mut self,
        account_id: &AccountId,
        token_id: &AccountId,
        amount: Balance) {
            let mut account = self.internal_unwrap_or_default_account(account_id);
            assert!(account.tokens.get(token_id).is_none(), "Token has already registered!");
            assert!(amount > 0, "Amount must be greater than 0!" );
            account.tokens.insert(token_id, &amount);
            self.internal_save_account(account_id, account);
    }
}

#[cfg(test)]
mod tests {

    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance, MockedBlockchain};
    use super::*;

    const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000;
    

    fn setup_contract() -> (VMContextBuilder, Contract) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).attached_deposit(ONE_NEAR).build());
        let contract = Contract::new(accounts(0));
        (context, contract)
    }


    #[test]
    fn test_deposit_token() {
        let token_id = accounts(3);
        let (_, mut contract) = setup_contract();
        let amount: Balance = 10000;
        let account_id = accounts(1);

        contract.storage_deposit(Some(account_id), Some(false));

        let  account_id = accounts(1);

        contract.internal_save_information_to_contract(&account_id.to_string(), &token_id.to_string(), amount);
        assert!(contract.accounts.get(&account_id.to_string()).unwrap().tokens.get(&token_id.to_string()).unwrap() == 10000, "TEST FAILED!!!!")
    }

    #[test]
    #[should_panic("ERR_TRANSFER_AMOUNT_EQUAL_ZERO")]
    fn test_deposit_token_with_zero_amount() {
        let token_id = accounts(3);
        let (_, mut contract) = setup_contract();
        let amount: Balance = 0;
        let account_id = accounts(1);

        contract.storage_deposit(Some(account_id), Some(false));

        let  account_id = accounts(1);

        contract.internal_save_information_to_contract(&account_id.to_string(), &token_id.to_string(), amount);
        
    }
}