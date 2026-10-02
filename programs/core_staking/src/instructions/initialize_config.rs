use crate::{core_accounts::load_collection, error::StakingError, *};
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};
use mpl_core::{
    fetch_collection_plugin,
    instructions::{
        AddCollectionPluginV1CpiBuilder, ApproveCollectionPluginAuthorityV1CpiBuilder,
        UpdateCollectionPluginV1CpiBuilder,
    },
    types::{Attribute, Attributes, Plugin, PluginAuthority, PluginType},
};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    /// CHECK: Core owner and collection layout checked; signer must be update authority.
    #[account(mut, owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,
    #[account(init, payer = admin, space = 8 + StakeConfig::INIT_SPACE,
        seeds = [CONFIG_SEED, collection.key().as_ref()], bump)]
    pub config: Account<'info, StakeConfig>,
    #[account(init, payer = admin, seeds = [REWARD_MINT_SEED, config.key().as_ref()], bump,
        mint::decimals = REWARD_DECIMALS, mint::authority = config)]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: fixed, executable Core program.
    #[account(address = mpl_core::ID, executable)]
    pub core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeConfig<'info> {
    pub fn initialize_config(
        &mut self,
        min_lock_seconds: i64,
        reward_rate: u64,
        burn_bonus: u64,
        bumps: InitializeConfigBumps,
    ) -> Result<()> {
        require!(min_lock_seconds >= 0, StakingError::NegativeLockPeriod);
        require!(
            reward_rate > 0 && burn_bonus > 0,
            StakingError::InvalidRewards
        );
        let collection = load_collection(&self.collection)?;
        require_keys_eq!(
            collection.update_authority,
            self.admin.key(),
            StakingError::NotCollectionAuthority
        );
        self.config.set_inner(StakeConfig {
            admin: self.admin.key(),
            collection: self.collection.key(),
            min_lock_seconds,
            total_staked: 0,
            reward_mint: self.reward_mint.key(),
            reward_rate,
            burn_bonus,
            bump: bumps.config,
            reward_mint_bump: bumps.reward_mint,
        });
        let delegate = PluginAuthority::Address {
            address: self.config.key(),
        };
        let existing =
            fetch_collection_plugin::<Attributes>(&self.collection, PluginType::Attributes);
        let mut attrs = existing
            .as_ref()
            .map(|(_, attrs, _)| attrs.clone())
            .unwrap_or(Attributes {
                attribute_list: vec![],
            });
        attrs
            .attribute_list
            .retain(|a| a.key != TOTAL_STAKED_ATTRIBUTE);
        attrs.attribute_list.push(Attribute {
            key: TOTAL_STAKED_ATTRIBUTE.into(),
            value: "0".into(),
        });
        if existing.is_ok() {
            UpdateCollectionPluginV1CpiBuilder::new(&self.core_program)
                .collection(&self.collection)
                .payer(&self.admin)
                .authority(Some(&self.admin))
                .system_program(&self.system_program)
                .plugin(Plugin::Attributes(attrs))
                .invoke()?;
            ApproveCollectionPluginAuthorityV1CpiBuilder::new(&self.core_program)
                .collection(&self.collection)
                .payer(&self.admin)
                .authority(Some(&self.admin))
                .system_program(&self.system_program)
                .plugin_type(PluginType::Attributes)
                .new_authority(delegate)
                .invoke()?;
        } else {
            AddCollectionPluginV1CpiBuilder::new(&self.core_program)
                .collection(&self.collection)
                .payer(&self.admin)
                .authority(Some(&self.admin))
                .system_program(&self.system_program)
                .plugin(Plugin::Attributes(attrs))
                .init_authority(delegate)
                .invoke()?;
        }
        Ok(())
    }
}
