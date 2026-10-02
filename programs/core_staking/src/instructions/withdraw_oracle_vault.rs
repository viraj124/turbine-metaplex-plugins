use crate::{core_accounts::load_collection, error::StakingError, *};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct WithdrawOracleVault<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [CONFIG_SEED, collection.key().as_ref()], bump = config.bump, has_one = collection)]
    pub config: Account<'info, StakeConfig>,
    /// CHECK: Core collection layout and current authority checked in handler.
    #[account(owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,
    #[account(mut, seeds = [ORACLE_VAULT_SEED, config.key().as_ref()], bump = oracle_vault.bump,
        has_one = config)]
    pub oracle_vault: Account<'info, OracleVault>,
}

impl<'info> WithdrawOracleVault<'info> {
    pub fn withdraw_oracle_vault(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, StakingError::InvalidAmount);
        require_keys_eq!(
            load_collection(&self.collection)?.update_authority,
            self.admin.key(),
            StakingError::NotCollectionAuthority
        );
        let vault = self.oracle_vault.to_account_info();
        let reserve = Rent::get()?.minimum_balance(vault.data_len());
        require!(
            amount <= vault.lamports().saturating_sub(reserve),
            StakingError::InsufficientVaultFunds
        );
        vault.sub_lamports(amount)?;
        self.admin.add_lamports(amount)?;
        Ok(())
    }
}
