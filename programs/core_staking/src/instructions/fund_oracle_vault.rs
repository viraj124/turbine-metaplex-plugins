use crate::{error::StakingError, *};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct FundOracleVault<'info> {
    #[account(mut)]
    pub funder: Signer<'info>,
    #[account(seeds = [CONFIG_SEED, config.collection.as_ref()], bump = config.bump)]
    pub config: Account<'info, StakeConfig>,
    #[account(mut, seeds = [ORACLE_VAULT_SEED, config.key().as_ref()], bump = oracle_vault.bump,
        has_one = config)]
    pub oracle_vault: Account<'info, OracleVault>,
    pub system_program: Program<'info, System>,
}

impl<'info> FundOracleVault<'info> {
    pub fn fund_oracle_vault(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, StakingError::InvalidAmount);
        transfer(
            CpiContext::new(
                self.system_program.key(),
                Transfer {
                    from: self.funder.to_account_info(),
                    to: self.oracle_vault.to_account_info(),
                },
            ),
            amount,
        )
    }
}
