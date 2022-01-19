//! State structs.

use anchor_lang::prelude::*;
use anchor_lang::solana_program;

#[account]
#[derive(Default, Debug, PartialEq)]
pub struct SmartWallet {
    /// Base used to derive.
    pub base: Pubkey,
    /// Bump seed for deriving PDA seeds.
    pub bump: u8,

    /// Minimum number of owner approvals needed to sign a transaction.
    pub threshold: u64,
    /// Minimum delay between approval and execution.
    pub minimum_delay: i64,
    /// Time after the ETA until a transaction expires.
    pub grace_period: i64,

    /// Sequence of the ownership set.
    pub owner_set_seqno: u32,
    /// Total number of [Transaction]s on this [SmartWallet].
    pub num_transactions: u64,

    /// Owners of the [SmartWallet].
    pub owners: Vec<Pubkey>,

    /// Extra space for program upgrades.
    pub reserved: [u64; 16],
}

impl SmartWallet {
    /// Computes the space a [SmartWallet] uses.
    pub fn space(max_owners: u8) -> usize {
        4 // Anchor discriminator
            + std::mem::size_of::<SmartWallet>()
            + 4 // 4 = the Vec discriminator
            + std::mem::size_of::<Pubkey>() * (max_owners as usize)
    }
}

#[account]
#[derive(Debug, Default, PartialEq)]
pub struct Transaction {
    /// The [SmartWallet] account this transaction belongs to.
    pub smart_wallet: Pubkey,
    /// The auto-incremented integer index of the transaction.
    /// All transactions on the [SmartWallet] can be looked up via this index,
    /// allowing for easier browsing of a wallet's historical transactions.
    pub index: u64,
    /// Bump seed.
    pub bump: u8,

    /// The proposer of the [Transaction].
    pub proposer: Pubkey,
    /// The instruction.
    pub instructions: Vec<TXInstruction>,
    /// `signers[index]` is true iff `[SmartWallet]::owners[index]` signed the transaction.
    pub signers: Vec<bool>,
    /// Owner set sequence number.
    pub owner_set_seqno: u32,
    /// Estimated time transaction will be executed
    pub eta: i64,

    /// The account that executed the [Transaction].
    pub executor: Pubkey,
    /// When the transaction was executed. -1 if not executed.
    pub executed_at: i64,
}

impl Transaction {
    /// Computes the space a [Transaction] uses.
    pub fn new(size: usize) -> usize {
        std::mem::size_of::<Transaction>() * size
    }
    /// Computes the space a [Transaction] uses.
    pub fn space(blank_xacts: Vec<TXInstruction>) -> usize {
        4  // Anchor discriminato
            + std::mem::size_of::<Transaction>()
            + 4 // Vec discriminator
            // + blank_xact.space()
            + (blank_xacts.iter().map(|ix| ix.space()).sum::<usize>())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq)]
pub struct StakeData {
    pub duration: i32,
    pub genesis_epoch: Vec<u8>,
    pub name: Vec<u8>,
    pub reward_pot: i64,
    pub protected_gids: Vec<u16>,
    pub uuid: Vec<u8>,
}

/// Instruction.
#[account]
#[derive(Debug, Default, PartialEq)]
pub struct Stake {
    pub bump: u8,
    pub duration: i32,
    pub genesis_epoch: Vec<u8>,
    pub name: Vec<u8>,
    pub reward_pot: i64,
    pub protected_gids: Vec<u16>,
    pub uuid: Vec<u8>,
}

impl Stake {
    pub fn space(protected_gids: usize) -> usize {
        8 +
            1 + // bump
            4 + // reward_tender
            4 + (8 * 1) + // gen epoch
            4 + (32 * 1) + // 32 char name utf-8
            8 + // reward_pot
            4 + (protected_gids * 2) + // protected_gids
            4 + 36 // 36 char bytes of uuid string

    }
}
/// Instruction.
#[account]
#[derive(Debug, Default, PartialEq)]
pub struct Ticket {
    pub enrollment_epoch: Vec<u8>,
    pub bump: u8,
    pub gid: u16,
    pub mint: Pubkey,
    pub owner: Pubkey,
}

impl Ticket {
    pub fn space() -> usize {
        8 +
            4 + (8 * 1) +
            1 +
            2 +
            4 + 32 +
            4 + 32
    }
}

/// Instruction.
#[account]
#[derive(Debug, Default, PartialEq)]
pub struct Rollup {
    pub bump: u8,
    pub timestamp: Vec<u8>,
    pub gid: u16,
    pub mints: u32,
}

impl Rollup {
    pub fn space() -> usize {
        8 +
            1 +
            4 + (8 * 1) +
            2 +
            4
    }
}

/// Instruction.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug, Default, PartialEq)]
pub struct TXInstruction {
    /// Pubkey of the instruction processor that executes this instruction
    pub program_id: Pubkey,
    /// Metadata for what accounts should be passed to the instruction processor
    pub keys: Vec<TXAccountMeta>,
    /// Opaque data passed to the instruction processor
    pub data: Vec<u8>,
}

impl TXInstruction {
    /// Computes the space a [Transaction] uses.
    pub fn blank(&self, size: usize) -> usize {
        TXInstruction::space(self) * size
    }
    /// Space that a [TXInstruction] takes up.
    pub fn space(&self) -> usize {
        std::mem::size_of::<Pubkey>()
            + (self.keys.len() as usize) * std::mem::size_of::<TXAccountMeta>()
            + (self.data.len() as usize)
    }
}

/// Account metadata used to define [TXInstruction]s
#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Copy, Clone)]
pub struct TXAccountMeta {
    /// An account's public key
    pub pubkey: Pubkey,
    /// True if an Instruction requires a Transaction signature matching `pubkey`.
    pub is_signer: bool,
    /// True if the `pubkey` can be loaded as a read-write account.
    pub is_writable: bool,
}

impl From<&TXInstruction> for solana_program::instruction::Instruction {
    fn from(tx: &TXInstruction) -> solana_program::instruction::Instruction {
        solana_program::instruction::Instruction {
            program_id: tx.program_id,
            accounts: tx.keys.clone().into_iter().map(Into::into).collect(),
            data: tx.data.clone(),
        }
    }
}

impl From<TXAccountMeta> for solana_program::instruction::AccountMeta {
    fn from(
        TXAccountMeta {
            pubkey,
            is_signer,
            is_writable,
        }: TXAccountMeta,
    ) -> solana_program::instruction::AccountMeta {
        solana_program::instruction::AccountMeta {
            pubkey,
            is_signer,
            is_writable,
        }
    }
}

/// Type of Subaccount.
#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum SubaccountType {
    Derived = 0,
    OwnerInvoker = 1,
}

impl Default for SubaccountType {
    fn default() -> Self {
        SubaccountType::Derived
    }
}

#[account]
#[derive(Default, Debug, PartialEq)]
pub struct SubaccountInfo {
    /// Smart wallet of the sub-account.
    pub smart_wallet: Pubkey,
    /// Type of sub-account.
    pub subaccount_type: SubaccountType,
    /// Index of the sub-account.
    pub index: u64,
}
