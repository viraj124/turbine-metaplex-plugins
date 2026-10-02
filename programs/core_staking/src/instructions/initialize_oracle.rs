use crate::{core_accounts::load_collection, error::StakingError, *};
use anchor_lang::prelude::*;
use mpl_core::{
    instructions::AddCollectionExternalPluginAdapterV1CpiBuilder,
    types::{
        ExternalCheckResult, ExternalPluginAdapterInitInfo, HookableLifecycleEvent, OracleInitInfo,
        PluginAuthority, ValidationResultsOffset,
    },
};

#[derive(Accounts)]
pub struct InitializeOracle<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED, collection.key().as_ref()], bump = config.bump,
        has_one = collection)]
    pub config: Account<'info, StakeConfig>,
    /// CHECK: Core-owned collection; current update authority must sign.
    #[account(mut, owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,
    #[account(init, payer = admin, space = 8 + TransferOracle::INIT_SPACE,
        seeds = [ORACLE_SEED, config.key().as_ref()], bump)]
    pub oracle: Account<'info, TransferOracle>,
    #[account(init, payer = admin, space = 8 + OracleVault::INIT_SPACE,
        seeds = [ORACLE_VAULT_SEED, config.key().as_ref()], bump)]
    pub oracle_vault: Account<'info, OracleVault>,
    /// CHECK: fixed, executable Core program.
    #[account(address = mpl_core::ID, executable)]
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeOracle<'info> {
    pub fn initialize_oracle(
        &mut self,
        open_hour: u8,
        close_hour: u8,
        crank_reward: u64,
        bumps: InitializeOracleBumps,
    ) -> Result<()> {
        require!(
            open_hour < close_hour && close_hour <= 24,
            StakingError::InvalidTransferHours
        );
        require!(crank_reward > 0, StakingError::InvalidAmount);
        require_keys_eq!(
            load_collection(&self.collection)?.update_authority,
            self.admin.key(),
            StakingError::NotCollectionAuthority
        );
        let now = Clock::get()?.unix_timestamp;
        self.oracle.set_inner(TransferOracle {
            core_validation: LifecycleValidation::new(ValidationState::Rejected),
            validation: LifecycleValidation::new(ValidationState::Rejected),
            config: self.config.key(),
            open_hour,
            close_hour,
            updated_at: now,
            crank_reward,
            last_rewarded_boundary: -1,
            bump: bumps.oracle,
        });
        self.oracle.refresh(now)?;
        self.oracle_vault.set_inner(OracleVault {
            config: self.config.key(),
            bump: bumps.oracle_vault,
        });
        AddCollectionExternalPluginAdapterV1CpiBuilder::new(&self.core_program)
            .collection(&self.collection)
            .payer(&self.admin)
            .authority(Some(&self.admin))
            .system_program(&self.system_program)
            .init_info(ExternalPluginAdapterInitInfo::Oracle(OracleInitInfo {
                base_address: self.oracle.key(),
                init_plugin_authority: Some(PluginAuthority::Address {
                    address: self.config.key(),
                }),
                lifecycle_checks: vec![(
                    HookableLifecycleEvent::Transfer,
                    ExternalCheckResult { flags: 4 },
                )],
                base_address_config: None,
                results_offset: Some(ValidationResultsOffset::Anchor),
            }))
            .invoke()?;
        Ok(())
    }
}
