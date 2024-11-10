use crate::primitives::ScrollAccountInfo;
use reth_scroll_primitives::ScrollPostExecutionContext;
use revm::{
    db::{AccountStatus, BundleAccount, StorageWithOriginalValues},
    interpreter::primitives::U256,
};

/// The scroll account bundle. Originally defined in [`BundleAccount`], a
/// scroll version of the bundle is needed for the [`crate::states::ScrollBundleState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollBundleAccount {
    /// The current account's information
    pub info: Option<ScrollAccountInfo>,
    /// The original account's information
    pub original_info: Option<ScrollAccountInfo>,
    /// Contains both original and present state.
    /// When extracting changeset we compare if original value is different from present value.
    /// If it is different we add it to changeset.
    ///
    /// If Account was destroyed we ignore original value and compare present state with
    /// [`U256::ZERO`].
    pub storage: StorageWithOriginalValues,
    /// Account status.
    pub status: AccountStatus,
}

impl From<(BundleAccount, &ScrollPostExecutionContext)> for ScrollBundleAccount {
    fn from((account, context): (BundleAccount, &ScrollPostExecutionContext)) -> Self {
        let info = account.info.map(|info| (info, context).into());
        let original_info = account.original_info.map(|info| (info, context).into());
        Self { info, original_info, storage: account.storage, status: account.status }
    }
}

impl ScrollBundleAccount {
    /// Creates a [`ScrollBundleAccount`].
    pub const fn new(
        original_info: Option<ScrollAccountInfo>,
        present_info: Option<ScrollAccountInfo>,
        storage: StorageWithOriginalValues,
        status: AccountStatus,
    ) -> Self {
        Self { info: present_info, original_info, storage, status }
    }

    /// The approximate size of changes needed to store this account.
    /// `1 + storage_len`
    pub fn size_hint(&self) -> usize {
        1 + self.storage.len()
    }

    /// Return storage slot if it exists.
    ///
    /// In case we know that account is newly created or destroyed, return `Some(U256::ZERO)`
    pub fn storage_slot(&self, slot: U256) -> Option<U256> {
        let slot = self.storage.get(&slot).map(|s| s.present_value);
        if slot.is_some() {
            slot
        } else if self.status.is_storage_known() {
            Some(U256::ZERO)
        } else {
            None
        }
    }

    /// Fetch account info if it exists.
    pub fn account_info(&self) -> Option<ScrollAccountInfo> {
        self.info.clone()
    }

    /// Was this account destroyed.
    pub fn was_destroyed(&self) -> bool {
        self.status.was_destroyed()
    }

    /// Return true of account info was changed.
    pub fn is_info_changed(&self) -> bool {
        self.info != self.original_info
    }

    /// Return true if contract was changed
    pub fn is_contract_changed(&self) -> bool {
        self.info.as_ref().map(|a| a.code_hash) != self.original_info.as_ref().map(|a| a.code_hash)
    }
}
