use crate::{error::StakingError, *};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct UpdateOracle<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(seeds = [CONFIG_SEED, config.collection.as_ref()], bump = config.bump)]
    pub config: Account<'info, StakeConfig>,
    #[account(mut, seeds = [ORACLE_SEED, config.key().as_ref()], bump = oracle.bump, has_one = config)]
    pub oracle: Account<'info, TransferOracle>,
    #[account(mut, seeds = [ORACLE_VAULT_SEED, config.key().as_ref()], bump = oracle_vault.bump,
        has_one = config)]
    pub oracle_vault: Account<'info, OracleVault>,
}

impl<'info> UpdateOracle<'info> {
    pub fn update_oracle(&mut self) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        self.oracle.refresh(now)?;
        if let Some(boundary) = self.oracle.reward_boundary(now)? {
            let amount = self.oracle.crank_reward;
            let vault = self.oracle_vault.to_account_info();
            let reserve = Rent::get()?.minimum_balance(vault.data_len());
            let spendable = vault.lamports().saturating_sub(reserve);
            if boundary > self.oracle.last_rewarded_boundary && spendable >= amount {
                self.oracle.last_rewarded_boundary = boundary;
                vault.sub_lamports(amount)?;
                self.caller.add_lamports(amount)?;
                msg!("oracle boundary {}: paid {} lamports", boundary, amount);
            }
        }
        require!(
            self.oracle.core_validation.transfer == ValidationState::Rejected,
            StakingError::InvalidCollectionStats
        );
        Ok(())
    }
}
