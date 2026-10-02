use crate::{
    core_accounts::{
        load_asset, require_owned_member, require_staking_freeze, update_collection_count,
    },
    error::StakingError,
    rewards::mint_rewards,
    *,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_core::{
    instructions::{BurnV1CpiBuilder, UpdatePluginV1CpiBuilder},
    types::{FreezeDelegate, Plugin},
};

#[derive(Accounts)]
pub struct BurnStakedNft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [CONFIG_SEED, collection.key().as_ref()], bump = config.bump,
        has_one = collection, has_one = reward_mint)]
    pub config: Account<'info, StakeConfig>,
    /// CHECK: Core-owned collection, tied to config.
    #[account(mut, owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: Core-owned asset; its layout, ownership, membership and freeze are checked.
    #[account(mut, owner = mpl_core::ID)]
    pub asset: UncheckedAccount<'info>,
    #[account(mut, close = owner, seeds = [STAKE_SEED, asset.key().as_ref()], bump = stake_record.bump,
        has_one = owner @ StakingError::NotAssetOwner, has_one = asset, has_one = config)]
    pub stake_record: Account<'info, StakeRecord>,
    #[account(mut, seeds = [REWARD_MINT_SEED, config.key().as_ref()], bump = config.reward_mint_bump,
        mint::authority = config, mint::decimals = REWARD_DECIMALS)]
    pub reward_mint: Account<'info, Mint>,
    #[account(init_if_needed, payer = owner, associated_token::mint = reward_mint,
        associated_token::authority = owner)]
    pub rewards_ata: Account<'info, TokenAccount>,
    /// CHECK: fixed, executable Core program.
    #[account(address = mpl_core::ID, executable)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> BurnStakedNft<'info> {
    pub fn burn_staked_nft(&mut self) -> Result<()> {
        let asset = load_asset(&self.asset)?;
        require_owned_member(&asset, &self.owner.key(), &self.config.collection)?;
        require_staking_freeze(&self.asset, self.config.key())?;
        let now = Clock::get()?.unix_timestamp;
        self.stake_record
            .require_unlocked(now, self.config.min_lock_seconds)?;
        let amount = self
            .stake_record
            .pending_rewards(now, self.config.reward_rate)?
            .checked_add(self.config.burn_bonus)
            .ok_or(StakingError::ArithmeticOverflow)?;
        let collection = self.collection.key();
        let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, collection.as_ref(), &[self.config.bump]]];
        UpdatePluginV1CpiBuilder::new(&self.core_program)
            .asset(&self.asset)
            .collection(Some(&self.collection))
            .payer(&self.owner)
            .authority(Some(&self.config.to_account_info()))
            .system_program(&self.system_program)
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: false }))
            .invoke_signed(signer_seeds)?;
        BurnV1CpiBuilder::new(&self.core_program)
            .asset(&self.asset)
            .collection(Some(&self.collection))
            .payer(&self.owner)
            .authority(Some(&self.config.to_account_info()))
            .system_program(Some(&self.system_program))
            .invoke_signed(signer_seeds)?;
        let new_count = self
            .config
            .total_staked
            .checked_sub(1)
            .ok_or(StakingError::InvalidCollectionStats)?;
        update_collection_count(
            &self.core_program,
            &self.collection,
            &self.owner,
            &self.config,
            &self.system_program,
            new_count,
        )?;
        self.config.total_staked = new_count;
        mint_rewards(
            &self.config,
            self.reward_mint.to_account_info(),
            self.rewards_ata.to_account_info(),
            self.token_program.key(),
            amount,
        )?;
        self.stake_record.last_claimed_at = now;
        msg!("burn_staked_nft: {} reward base units", amount);
        Ok(())
    }
}
