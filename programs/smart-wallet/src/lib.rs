//! Multisig Solana wallet with Timelock capabilities.
//!
//! This program can be used to allow a smart wallet to govern anything a regular
//! [Pubkey] can govern. One can use the smart wallet as a BPF program upgrade
//! authority, a mint authority, etc.
//!
//! To use, one must first create a [SmartWallet] account, specifying two important
//! parameters:
//!
//! 1. Owners - the set of addresses that sign transactions for the smart wallet.
//! 2. Threshold - the number of signers required to execute a transaction.
//! 3. Minimum Delay - the minimum amount of time that must pass before a [Transaction]
//!                    can be executed. If 0, this is ignored.
//!
//! Once the [SmartWallet] account is created, one can create a [Transaction]
//! account, specifying the parameters for a normal Solana instruction.
//!
//! To sign, owners should invoke the [smart_wallet::approve] instruction, and finally,
//! [smart_wallet::execute_transaction], once enough (i.e. [SmartWallet::threshold]) of the owners have
//! signed.
#![deny(rustdoc::all)]
#![allow(rustdoc::missing_doc_code_examples)]

use anchor_lang::prelude::*;
use anchor_spl::token::{Token};
use anchor_lang::solana_program;
use anchor_lang::Key;
use std::convert::Into;
use std::vec::Vec;
use vipers::invariant;
use vipers::unwrap_int;
use vipers::unwrap_or_err;
use vipers::validate::Validate;

mod events;
mod smart_wallet_utils;
mod state;
mod transaction;
mod validators;

pub use events::*;
pub use state::*;

/// Number of seconds in a day.
pub const SECONDS_PER_DAY: i64 = 60 * 60 * 24;

/// Maximum timelock delay.
pub const MAX_DELAY_SECONDS: i64 = 365 * SECONDS_PER_DAY;

/// Default number of seconds until a transaction expires.
pub const DEFAULT_GRACE_PERIOD: i64 = 14 * SECONDS_PER_DAY;

/// Constant declaring that there is no ETA of the transaction.
pub const NO_ETA: i64 = -1;

declare_id!("9UgyDew11rjMzcrWa8BMNQVkPSuU2Gv33YocZhfMQVuR");

#[program]
/// Goki smart wallet program.
pub mod smart_wallet {
    use super::*;

    /// Initializes a new [SmartWallet] account with a set of owners and a threshold.
    #[access_control(ctx.accounts.validate())]
    pub fn create_smart_wallet(
        ctx: Context<CreateSmartWallet>,
        bump: u8,
        max_owners: u8,
        owners: Vec<Pubkey>,
        threshold: u64,
        minimum_delay: i64,
    ) -> ProgramResult {
        invariant!(minimum_delay >= 0, "delay must be positive");
        require!(minimum_delay < MAX_DELAY_SECONDS, DelayTooHigh);

        invariant!((max_owners as usize) >= owners.len(), "max_owners");

        let smart_wallet = &mut ctx.accounts.smart_wallet;
        smart_wallet.base = ctx.accounts.base.key();
        smart_wallet.bump = bump;

        smart_wallet.threshold = threshold;
        smart_wallet.minimum_delay = minimum_delay;
        smart_wallet.grace_period = DEFAULT_GRACE_PERIOD;

        smart_wallet.owner_set_seqno = 0;
        smart_wallet.num_transactions = 0;

        smart_wallet.owners = owners.clone();

        /*
        emit!(WalletCreateEvent {
            smart_wallet: ctx.accounts.smart_wallet.key(),
            owners,
            threshold,
            minimum_delay,
            timestamp: Clock::get()?.unix_timestamp
        });
        */
        Ok(())
    }

    /// Registers participant.
    pub fn create_stake(
        ctx: Context<CreateStake>,
        bump: u8,
        abs_index: u64,
        stake_data: StakeData,
    ) -> ProgramResult {
        let stake_account = &mut ctx.accounts.stake;
        stake_account.bump = bump;
        stake_account.genesis_epoch = stake_data.genesis_epoch;
        stake_account.name = stake_data.name;
        stake_account.reward_pot = stake_data.reward_pot;
        stake_account.duration = stake_data.duration;
        stake_account.protected_gids = stake_data.protected_gids;
        stake_account.uuid = stake_data.uuid;

        // msg!("Stake genesis for {:?} with {:?} genesis_epoch", stake_account.key(), stake_account.genesis_epoch);
        // msg!("{:?} duration", stake_account.duration);
        emit!(CreateStakeEvent {
            smart_wallet: ctx.accounts.smart_wallet.key(),
            stake: ctx.accounts.stake.key(),
        });
        Ok(())
    }
    /// inits rollup account.
    pub fn rollup_entity(
        ctx: Context<RollupEntityInit>,
        bump: u8,
        gid: u16,
    ) -> ProgramResult {
        let enrollment_epoch: i64 = Clock::get()?.unix_timestamp;

        let rollup_account = &mut ctx.accounts.rollup;
        rollup_account.timestamp = enrollment_epoch.to_le_bytes().to_vec();
        rollup_account.bump = bump;
        rollup_account.gid = gid;
        rollup_account.mints = 0;
        require!(rollup_account.gid == gid, NoGIDJack);

        Ok(())
    }
    /// Registers participant.
    pub fn register_entity(
        ctx: Context<RegisterEntity>,
        bump: u8,
        gid: u16,
    ) -> ProgramResult {
        let enrollment_epoch: i64 = Clock::get()?.unix_timestamp;
        let ticket_account = &mut ctx.accounts.ticket;
        let rollup_account = &mut ctx.accounts.rollup;
        require!(rollup_account.gid == gid, NoGIDJack);

        ticket_account.enrollment_epoch = enrollment_epoch.to_le_bytes().to_vec();
        ticket_account.bump = bump;
        ticket_account.gid = gid;
        ticket_account.mint = ctx.accounts.mint.key();
        ticket_account.owner = ctx.accounts.owner.key();
        rollup_account.mints = unwrap_int!(rollup_account.mints.checked_add(1));
        msg!("{:?}", rollup_account.mints);

        Ok(())
    }

    /// claims all in participant.
    pub fn claim_entities(
        ctx: Context<ClaimEntities>,
        bump: u8,
    ) -> ProgramResult {
        let reset_epoch: i64 = Clock::get()?.unix_timestamp;
        require!(ctx.accounts.rollup.bump == bump, InvalidBump);

        let rollup_account = &mut ctx.accounts.rollup;
        // rollup_account.timestamp = reset_epoch.to_le_bytes().to_vec();
        require!(!ctx.accounts.stake.protected_gids.contains(&rollup_account.gid), ProtectedGid);

        let former_epoch = rollup_account.timestamp.clone();
        let duration = reset_epoch - i64::from_le_bytes(former_epoch.try_into().unwrap());
        let former_epoch = rollup_account.timestamp.clone();
        emit!(ClaimEntitiesEvent {
            smart_wallet: ctx.accounts.smart_wallet.key(),
            duration: duration.to_le_bytes().to_vec(),
            last_epoch: former_epoch,
            reset_epoch: reset_epoch.to_le_bytes().to_vec(),
            mints: rollup_account.mints,
            rollup: rollup_account.key(),
            stake: ctx.accounts.stake.key(),
            owner: ctx.accounts.owner.key(),
        });
        Ok(())
    }
    /// Updates participant.
    pub fn update_entity_by_owner(
        ctx: Context<UpdateEntityByOwner>,
        bump: u8,
    ) -> ProgramResult {
        let reset_epoch: i64 = Clock::get()?.unix_timestamp;
        let rollup_account = &mut ctx.accounts.rollup;
        let ticket_account = &mut ctx.accounts.ticket;
        let ata = anchor_spl::associated_token::get_associated_token_address(
            &ctx.accounts.owner.key(),
            &ctx.accounts.mint.key(),
        );
        require!(ata == ctx.accounts.mint_ata.key(), InvalidATA);
        require!(ticket_account.bump == bump, InvalidBump);

        rollup_account.mints = unwrap_int!(rollup_account.mints.checked_add(1));
        ticket_account.enrollment_epoch = reset_epoch.to_le_bytes().to_vec();

        Ok(())
    }
    /// Updates participant.
    pub fn update_entity(
        ctx: Context<UpdateEntity>,
        bump: u8,
        timestamp: Vec<u8>,
    ) -> ProgramResult {
        let _owner_index = ctx.accounts.smart_wallet.owner_index(ctx.accounts.smart_wallet_owner.key())?;
        let timestamp_i = i64::from_le_bytes(timestamp.try_into().unwrap());
        let ticket_account = &mut ctx.accounts.ticket;
        let rollup_account = &mut ctx.accounts.rollup;
        require!(ticket_account.bump == bump, InvalidBump);

        ticket_account.enrollment_epoch = timestamp_i.to_le_bytes().to_vec();
        rollup_account.timestamp = timestamp_i.to_le_bytes().to_vec();
        Ok(())
    }
    pub fn withdraw_entity_by_program(
        ctx: Context<WithdrawEntityByProgram>,
        bump: u8,
    ) -> ProgramResult {
        let _owner_index = ctx.accounts.smart_wallet.owner_index(ctx.accounts.smart_wallet_owner.key())?;
        // -1 is !false
        let reset_epoch: i64 = -1;
        // let rollup_account = &mut ctx.accounts.rollup;
        let ticket_account = &mut ctx.accounts.ticket;

        require!(ticket_account.bump == bump, InvalidBump);
        require!(ticket_account.mint == ctx.accounts.mint.key(), InvalidMint);
        // require!(!ctx.accounts.stake.protected_gids.contains(&ticket_account.gid), ProtectedGid);

        ticket_account.enrollment_epoch = reset_epoch.to_le_bytes().to_vec();
        emit!(WithdrawEntityEvent {
            smart_wallet: ctx.accounts.smart_wallet.key(),
            mint: ctx.accounts.mint.key(),
            ticket: ticket_account.key(),
            stake: ctx.accounts.stake.key(),
            owner: ctx.accounts.owner.key(),
        });

        Ok(())
    }
    pub fn withdraw_entity(
        ctx: Context<WithdrawEntity>,
        bump: u8,
    ) -> ProgramResult {
        let reset_epoch: i64 = 0;
        let rollup_account = &mut ctx.accounts.rollup;
        let ticket_account = &mut ctx.accounts.ticket;

        require!(ticket_account.bump == bump, InvalidBump);
        require!(ticket_account.mint == ctx.accounts.mint.key(), InvalidMint);
        require!(!ctx.accounts.stake.protected_gids.contains(&ticket_account.gid), ProtectedGid);

        rollup_account.mints = unwrap_int!(rollup_account.mints.checked_sub(1));
        ticket_account.enrollment_epoch = reset_epoch.to_le_bytes().to_vec();
        emit!(WithdrawEntityEvent {
            smart_wallet: ctx.accounts.smart_wallet.key(),
            mint: ctx.accounts.mint.key(),
            ticket: ticket_account.key(),
            stake: ctx.accounts.stake.key(),
            owner: ctx.accounts.owner.key(),
        });

        Ok(())
    }

    /// Executes ixs arg
    #[access_control(ctx.accounts.validate())]
    pub fn execute_ixs(
        ctx: Context<ExecuteInstructions>,
        index: u64,
        bump: u8,
        ixs: Vec<TXInstruction>,
    ) -> ProgramResult {
        let smart_wallet = &ctx.accounts.smart_wallet;
        let wallet_seeds: &[&[&[u8]]] = &[&[
            b"GokiSmartWalletDerived" as &[u8],
            &smart_wallet.key().to_bytes(),
            &index.to_le_bytes(),
            &[bump],
        ]];

        for ix in ixs.iter() {
            solana_program::program::invoke_signed(&(ix).into(), ctx.remaining_accounts, wallet_seeds)?;
        }
        Ok(())
    }
}

/// Accounts for [smart_wallet::create_smart_wallet].
#[derive(Accounts)]
#[instruction(bump: u8, max_owners: u8)]
pub struct CreateSmartWallet<'info> {
    /// Base key of the SmartWallet.
    pub base: Signer<'info>,

    /// The [SmartWallet] to create.
    #[account(
        init,
        seeds = [
            b"GokiSmartWallet".as_ref(),
            base.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = SmartWallet::space(max_owners),
    )]
    pub smart_wallet: Account<'info, SmartWallet>,

    /// Payer to create the smart_wallet.
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The [System] program.
    pub system_program: Program<'info, System>,
}

/// Accounts for [smart_wallet:append_transaction].
#[derive(Accounts)]
#[instruction(bump: u8, instructions: TXInstruction)]
pub struct AppendTransaction<'info> {
    /// The [SmartWallet].
    #[account(mut)]
    pub smart_wallet: Account<'info, SmartWallet>,
    /// The [Transaction].
    #[account(mut)]
    pub transaction: Account<'info, Transaction>,
    /// One of the smart_wallet owners. Checked in the handler.
    pub owner: Signer<'info>,
}

/// Accounts for [smart_wallet:append_transaction].
#[derive(Accounts)]
#[instruction(bump: u8, abs_index: u64, stake_data: StakeData)]
pub struct CreateStake<'info> {
    /// The [SmartWallet].
    #[account(mut)]
    pub smart_wallet: Account<'info, SmartWallet>,
    /// The [Ticket].
    #[account(
        init,
        seeds = [
            b"Stake".as_ref(),
            smart_wallet.key().to_bytes().as_ref(),
            abs_index.to_le_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = Stake::space(stake_data.protected_gids.len()),
    )]
    pub stake: Account<'info, Stake>,
    /// Payer to create the [Transaction].
    #[account(mut)]
    pub payer: Signer<'info>,
    /// The mint owner. Checked in the handler.
    pub owner: Signer<'info>,
    /// The [System] program.
    pub system_program: Program<'info, System>,
}
/// Accounts for [smart_wallet:append_transaction].
#[derive(Accounts)]
#[instruction(bump: u8, gid: u16)]
pub struct RollupEntityInit<'info> {
    /// Payer to create the [Transaction].
    // pub mint: UncheckedAccount<'info>,
    /// The [SmartWallet].
    #[account(mut)]
    pub smart_wallet: Account<'info, SmartWallet>,
    /// The [Ticket].
    #[account(
        init,
        seeds = [
            smart_wallet.key().to_bytes().as_ref(),
            owner.key().to_bytes().as_ref(),
            gid.to_le_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = Rollup::space(),
    )]
    pub rollup: Account<'info, Rollup>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// The mint owner. Checked in the handler.
    pub owner: Signer<'info>,
    /// The [System] program.
    pub system_program: Program<'info, System>,
}
/// Accounts for [smart_wallet:append_transaction].
#[derive(Accounts)]
#[instruction(bump: u8, gid: u16)]
pub struct RegisterEntity<'info> {
    /// Payer to create the [Transaction].
    // pub mint: UncheckedAccount<'info>,
    /// The [SmartWallet].
    #[account(mut)]
    pub smart_wallet: Account<'info, SmartWallet>,
    #[account(mut)]
    pub rollup: Account<'info, Rollup>,
    /// The [Ticket].
    #[account(
        init,
        seeds = [
            system_program.key().to_bytes().as_ref(),
            smart_wallet.key().to_bytes().as_ref(),
            mint.key().to_bytes().as_ref()
        ],
        bump,
        payer = payer,
        space = Ticket::space(),
    )]
    pub ticket: Account<'info, Ticket>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// The mint owner. Checked in the handler.
    pub owner: Signer<'info>,
    pub mint: UncheckedAccount<'info>,
    /// The [System] program.
    pub system_program: Program<'info, System>,
}

/// Accounts for [smart_wallet:append_transaction].
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct ClaimEntities<'info> {
    /// The [SmartWallet].
    #[account(mut)]
    pub smart_wallet: Account<'info, SmartWallet>,
    #[account(mut)]
    pub rollup: Account<'info, Rollup>,
    /// The [Ticket].
    #[account(mut)]
    pub stake: Account<'info, Stake>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// The mint owner. Checked in the handler.
    pub owner: Signer<'info>,
    /// The [System] program.
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct UpdateEntityByOwner<'info> {
    #[account(mut)]
    pub smart_wallet: Account<'info, SmartWallet>,
    #[account(mut)]
    pub ticket: Account<'info, Ticket>,
    #[account(mut)]
    pub rollup: Account<'info, Rollup>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut)]
    pub mint: UncheckedAccount<'info>,
    pub mint_ata: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
#[instruction(bump: u8, timestamp: Vec<u8>)]
pub struct UpdateEntity<'info> {
    #[account(mut)]
    pub smart_wallet: Account<'info, SmartWallet>,
    #[account(mut)]
    pub ticket: Account<'info, Ticket>,
    #[account(mut)]
    pub rollup: Account<'info, Rollup>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub smart_wallet_owner: Signer<'info>,
    pub mint: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}
/// Accounts for [smart_wallet:append_transaction].
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct WithdrawEntityByProgram<'info> {
    #[account(mut)]
    pub smart_wallet: Account<'info, SmartWallet>,
    #[account(mut)]
    pub stake: Account<'info, Stake>,
    #[account(mut)]
    pub ticket: Account<'info, Ticket>,
    #[account(mut)]
    pub rollup: Account<'info, Rollup>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub owner: UncheckedAccount<'info>,
    pub smart_wallet_owner: Signer<'info>,
    pub mint: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
#[instruction(bump: u8)]
pub struct WithdrawEntity<'info> {
    #[account(mut)]
    pub smart_wallet: Account<'info, SmartWallet>,
    #[account(mut)]
    pub stake: Account<'info, Stake>,
    #[account(mut)]
    pub ticket: Account<'info, Ticket>,
    #[account(mut)]
    pub rollup: Account<'info, Rollup>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub owner: Signer<'info>,
    pub mint: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

/// Accounts for [smart_wallet::execute_transaction].
#[derive(Accounts)]
pub struct ExecuteInstructions<'info> {
    /// The [SmartWallet].
    pub smart_wallet: Account<'info, SmartWallet>,
    /// The [Transaction] to execute.
    #[account(mut)]
    /// owners of the [SmartWallet].
    pub authority_a: Signer<'info>,
    #[account(mut)]
    pub authority_b: Signer<'info>,
}

#[error]
pub enum ErrorCode {
    #[msg("The given owner is not part of this smart wallet.")]
    InvalidOwner,
    #[msg("Estimated execution block must satisfy delay.")]
    InvalidETA,
    #[msg("Delay greater than the maximum.")]
    DelayTooHigh,
    #[msg("Not enough owners signed this transaction.")]
    NotEnoughSigners,
    #[msg("Transaction is past the grace period.")]
    TransactionIsStale,
    #[msg("Transaction hasn't surpassed time lock.")]
    TransactionNotReady,
    #[msg("The given transaction has already been executed.")]
    AlreadyExecuted,
    #[msg("Threshold must be less than or equal to the number of owners.")]
    InvalidThreshold,
    #[msg("Owner set has changed since the creation of the transaction.")]
    OwnerSetChanged,
    #[msg("Invalid bump seed.")]
    InvalidBump,
    #[msg("Invalid Mint.")]
    InvalidMint,
    #[msg("Protected GID.")]
    ProtectedGid,
    #[msg("Qualified Hijack.")]
    NoJack,
    #[msg("Qualified GID Hijack.")]
    NoGIDJack,
    #[msg("Inconsistent Timestamp.")]
    DisingenuousUpdate,
    #[msg("Invalid Mint ATA.")]
    InvalidATA,
}
