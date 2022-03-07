use crate::*;

#[near_bindgen]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit( &mut self,account_id: Option<ValidAccountId>, registration_only: Option<bool>) -> StorageBalance {
        let amount = env::attached_deposit();
        let account_id = account_id
            .map(|a| a.into())
            .unwrap_or_else(|| env::predecessor_account_id());

        
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<U128>) -> StorageBalance {}

    #[allow(unused_variables)]
    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {}

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {}

    fn storage_balance_of(&self, account_id: ValidAccountId) -> Option<StorageBalance> {}
}