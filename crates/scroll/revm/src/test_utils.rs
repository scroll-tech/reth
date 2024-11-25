use crate::states::{
    ScrollAccountInfoRevert, ScrollAccountRevert, ScrollPlainStateReverts, ScrollStateChangeset,
};
use revm::db::{
    states::{reverts::AccountInfoRevert, PlainStateReverts, StateChangeset},
    AccountRevert,
};

// This conversion can cause a loss of information since performed without additional context.
impl From<StateChangeset> for ScrollStateChangeset {
    fn from(changeset: StateChangeset) -> Self {
        Self {
            accounts: changeset
                .accounts
                .into_iter()
                .map(|(add, acc)| (add, acc.map(Into::into)))
                .collect(),
            storage: changeset.storage,
            contracts: changeset.contracts,
        }
    }
}

// This conversion can cause a loss of information since performed without additional context.
impl From<PlainStateReverts> for ScrollPlainStateReverts {
    fn from(reverts: PlainStateReverts) -> Self {
        Self {
            accounts: reverts
                .accounts
                .into_iter()
                .map(|accounts| {
                    accounts.into_iter().map(|(add, acc)| (add, acc.map(Into::into))).collect()
                })
                .collect(),
            storage: reverts.storage,
        }
    }
}

// This conversion can cause a loss of information since performed without additional context.
impl From<AccountInfoRevert> for ScrollAccountInfoRevert {
    fn from(account: AccountInfoRevert) -> Self {
        match account {
            AccountInfoRevert::DoNothing => Self::DoNothing,
            AccountInfoRevert::DeleteIt => Self::DeleteIt,
            AccountInfoRevert::RevertTo(account) => Self::RevertTo(account.into()),
        }
    }
}

// This conversion can cause a loss of information since performed without additional context.
impl From<AccountRevert> for ScrollAccountRevert {
    fn from(account: AccountRevert) -> Self {
        Self {
            account: account.account.into(),
            storage: account.storage,
            previous_status: account.previous_status,
            wipe_storage: account.wipe_storage,
        }
    }
}
