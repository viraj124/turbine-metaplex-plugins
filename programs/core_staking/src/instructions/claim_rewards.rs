use crate::{
    core_accounts::{load_asset, require_owned_member, require_staking_freeze},
    error::StakingError,
    rewards::mint_rewards,
    *,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [CONFIG_SEED, collection.key().as_ref()], bump = config.bump,
        has_one = collection, has_one = reward_mint)]
    pub config: Account<'info, StakeConfig>,
    /// CHECK: Core-owned collection, tied to config.
    #[account(owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: Core-owned asset; its layout, ownership, membership and freeze are checked.
    #[account(owner = mpl_core::ID)]
    pub asset: UncheckedAccount<'info>,
    #[account(mut, seeds = [STAKE_SEED, asset.key().as_ref()], bump = stake_record.bump,
        has_one = owner @ StakingError::NotAssetOwner, has_one = asset, has_one = config)]
    pub stake_record: Account<'info, StakeRecord>,
    #[account(mut, seeds = [REWARD_MINT_SEED, config.key().as_ref()], bump = config.reward_mint_bump,
        mint::authority = config, mint::decimals = REWARD_DECIMALS)]
    pub reward_mint: Account<'info, Mint>,
    #[account(init_if_needed, payer = owner, associated_token::mint = reward_mint,
        associated_token::authority = owner)]
    pub rewards_ata: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> ClaimRewards<'info> {
    pub fn claim_rewards(&mut self) -> Result<()> {
        let asset = load_asset(&self.asset)?;
        require_owned_member(&asset, &self.owner.key(), &self.config.collection)?;
        require_staking_freeze(&self.asset, self.config.key())?;
        let now = Clock::get()?.unix_timestamp;
        let amount = self
            .stake_record
            .pending_rewards(now, self.config.reward_rate)?;
        require!(amount > 0, StakingError::NoRewards);
        mint_rewards(
            &self.config,
            self.reward_mint.to_account_info(),
            self.rewards_ata.to_account_info(),
            self.token_program.key(),
            amount,
        )?;
        self.stake_record.last_claimed_at = now;
        msg!("claim_rewards: {} reward base units", amount);
        Ok(())
    }
}
