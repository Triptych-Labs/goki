//! Events emitted.

use crate::*;

/// Emitted when a [SmartWallet] is created.
#[event]
pub struct WalletCreateEvent {
    #[index]
    pub smart_wallet: Pubkey,
    pub owners: Vec<Pubkey>,
    pub threshold: u64,
    pub minimum_delay: i64,
    pub timestamp: i64,
}
///
/// Emitted when a [Stake] is created.
#[event]
pub struct CreateStakeEvent {
    #[index]
    pub smart_wallet: Pubkey,
    pub stake: Pubkey,
}

/// Emitted when a [SmartWallet] is created.
#[event]
pub struct ClaimEntitiesEvent {
    #[index]
    pub smart_wallet: Pubkey,
    pub duration: Vec<u8>,
    pub last_epoch: Vec<u8>,
    pub reset_epoch: Vec<u8>,
    pub mints: u32,
    pub rollup: Pubkey,
    pub stake: Pubkey,
    pub owner: Pubkey,
}
#[event]
pub struct ClaimEntityEvent {
    #[index]
    pub smart_wallet: Pubkey,
    pub duration: Vec<u8>,
    pub mint: Pubkey,
    pub ticket: Pubkey,
    pub stake: Pubkey,
    pub owner: Pubkey,
}
/// Emitted when a [SmartWallet] is created.
#[event]
pub struct WithdrawEntityEvent {
    #[index]
    pub smart_wallet: Pubkey,
    pub mint: Pubkey,
    pub ticket: Pubkey,
    pub stake: Pubkey,
    pub owner: Pubkey,
}

/// Emitted when the owners of a [SmartWallet] are changed.
#[event]
pub struct WalletSetOwnersEvent {
    #[index]
    pub smart_wallet: Pubkey,
    pub owners: Vec<Pubkey>,
    pub timestamp: i64,
}

/// Emitted when the threshold of a [SmartWallet] is changed.
#[event]
pub struct WalletChangeThresholdEvent {
    #[index]
    pub smart_wallet: Pubkey,
    pub threshold: u64,
    pub timestamp: i64,
}

/// Emitted when a [Transaction] is proposed.
#[event]
pub struct TransactionCreateEvent {
    #[index]
    pub smart_wallet: Pubkey,
    #[index]
    pub transaction: Pubkey,
    pub proposer: Pubkey,
    /// Instructions associated with the [Transaction].
    pub instructions: Vec<TXInstruction>,
    pub eta: i64,
    pub timestamp: i64,
}

/// Emitted when a [Transaction] is approved.
#[event]
pub struct TransactionApproveEvent {
    #[index]
    pub smart_wallet: Pubkey,
    #[index]
    pub transaction: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a [Transaction] is unapproved.
#[event]
pub struct TransactionUnapproveEvent {
    #[index]
    pub smart_wallet: Pubkey,
    #[index]
    pub transaction: Pubkey,
    pub owner: Pubkey,
    pub timestamp: i64,
}

/// Emitted when a [Transaction] is executed.
#[event]
pub struct TransactionExecuteEvent {
    #[index]
    pub smart_wallet: Pubkey,
    #[index]
    pub transaction: Pubkey,
    pub executor: Pubkey,
    pub timestamp: i64,
}
