// Core NFT staking: Anchor 1.2 / TypeScript LiteSVM.
// Architecture follows turbine-amm-program; validation guidance from Safe Solana Builder.
// Risk: Critical (NFT burn, token mint authority, Core CPIs and a SOL crank vault).
// See security-checklist.md for trust assumptions and compatibility limits.
pub mod constants;
pub mod core_accounts;
pub mod error;
pub mod instructions;
pub mod rewards;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("5Du8dxpFpVZ38SN6CqDCMgcVUJwQWbLkao47ojJcx4Sy");

#[program]
pub mod core_staking {
    use super::*;

    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        min_lock_seconds: i64,
        reward_rate: u64,
        burn_bonus: u64,
    ) -> Result<()> {
        ctx.accounts
            .initialize_config(min_lock_seconds, reward_rate, burn_bonus, ctx.bumps)
    }

    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.stake(ctx.bumps.stake_record)
    }

    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.unstake()
    }

    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        ctx.accounts.claim_rewards()
    }

    pub fn burn_staked_nft(ctx: Context<BurnStakedNft>) -> Result<()> {
        ctx.accounts.burn_staked_nft()
    }

    pub fn initialize_oracle(
        ctx: Context<InitializeOracle>,
        open_hour: u8,
        close_hour: u8,
        crank_reward: u64,
    ) -> Result<()> {
        ctx.accounts
            .initialize_oracle(open_hour, close_hour, crank_reward, ctx.bumps)
    }

    pub fn update_oracle(ctx: Context<UpdateOracle>) -> Result<()> {
        ctx.accounts.update_oracle()
    }

    pub fn transfer_nft(ctx: Context<TransferNft>) -> Result<()> {
        ctx.accounts.transfer_nft()
    }

    pub fn fund_oracle_vault(ctx: Context<FundOracleVault>, amount: u64) -> Result<()> {
        ctx.accounts.fund_oracle_vault(amount)
    }

    pub fn withdraw_oracle_vault(ctx: Context<WithdrawOracleVault>, amount: u64) -> Result<()> {
        ctx.accounts.withdraw_oracle_vault(amount)
    }
}
