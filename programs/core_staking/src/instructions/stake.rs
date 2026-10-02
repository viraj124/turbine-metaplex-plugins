use anchor_lang::prelude::*;
use mpl_core::{
    fetch_asset_plugin,
    instructions::AddPluginV1CpiBuilder,
    types::{BurnDelegate, FreezeDelegate, Plugin, PluginAuthority, PluginType},
};

use crate::{
    core_accounts::{load_asset, require_owned_member, update_collection_count},
    error::StakingError,
    StakeConfig, StakeRecord, CONFIG_SEED, STAKE_SEED,
};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        bump = config.bump,
        has_one = collection,
    )]
    pub config: Account<'info, StakeConfig>,

    /// CHECK: pinned to `config.collection` by `has_one`. Core needs it
    /// writable to add a plugin to one of its assets.
    #[account(mut, owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,

    /// CHECK: owner is pinned to Core here, and `stake` checks the layout,
    /// the owner and the collection.
    #[account(mut, owner = mpl_core::ID)]
    pub asset: UncheckedAccount<'info>,

    #[account(
        init,
        payer = owner,
        space = 8 + StakeRecord::INIT_SPACE,
        seeds = [STAKE_SEED, asset.key().as_ref()],
        bump,
    )]
    pub stake_record: Account<'info, StakeRecord>,

    /// CHECK: address-checked against the Core program ID.
    #[account(address = mpl_core::ID, executable)]
    pub core_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Stake<'info> {
    pub fn stake(&mut self, bump: u8) -> Result<()> {
        let asset = load_asset(&self.asset)?;
        require_owned_member(&asset, &self.owner.key(), &self.config.collection)?;

        require!(
            fetch_asset_plugin::<FreezeDelegate>(&self.asset, PluginType::FreezeDelegate).is_err(),
            StakingError::AlreadyFreezeDelegated
        );

        require!(
            fetch_asset_plugin::<BurnDelegate>(&self.asset, PluginType::BurnDelegate).is_err(),
            StakingError::AlreadyBurnDelegated
        );
        AddPluginV1CpiBuilder::new(&self.core_program)
            .asset(&self.asset)
            .collection(Some(&self.collection))
            .payer(&self.owner)
            .authority(Some(&self.owner))
            .system_program(&self.system_program)
            .plugin(Plugin::BurnDelegate(BurnDelegate {}))
            .init_authority(PluginAuthority::Address {
                address: self.config.key(),
            })
            .invoke()?;

        AddPluginV1CpiBuilder::new(&self.core_program)
            .asset(&self.asset)
            .collection(Some(&self.collection))
            .payer(&self.owner)
            .authority(Some(&self.owner))
            .system_program(&self.system_program)
            .plugin(Plugin::FreezeDelegate(FreezeDelegate { frozen: true }))
            .init_authority(PluginAuthority::Address {
                address: self.config.key(),
            })
            .invoke()?;

        let staked_at = Clock::get()?.unix_timestamp;
        self.stake_record.set_inner(StakeRecord {
            owner: self.owner.key(),
            asset: self.asset.key(),
            config: self.config.key(),
            staked_at,
            last_claimed_at: staked_at,
            bump,
        });
        let new_count = self
            .config
            .total_staked
            .checked_add(1)
            .ok_or(StakingError::StakedCountOverflow)?;

        update_collection_count(
            &self.core_program,
            &self.collection,
            &self.owner,
            &self.config,
            &self.system_program,
            new_count,
        )?;
        self.config.total_staked = new_count;

        msg!("asset {} staked at {}", self.asset.key(), staked_at);
        Ok(())
    }
}
