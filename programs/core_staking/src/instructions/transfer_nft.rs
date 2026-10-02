use crate::{
    core_accounts::{load_asset, require_owned_member},
    error::StakingError,
    *,
};
use anchor_lang::prelude::*;
use mpl_core::instructions::TransferV1CpiBuilder;

#[derive(Accounts)]
pub struct TransferNft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(seeds = [CONFIG_SEED, collection.key().as_ref()], bump = config.bump, has_one = collection)]
    pub config: Account<'info, StakeConfig>,
    /// CHECK: Core-owned collection, bound to config.
    #[account(owner = mpl_core::ID)]
    pub collection: UncheckedAccount<'info>,
    /// CHECK: Core-owned asset; checked against owner and collection in handler.
    #[account(mut, owner = mpl_core::ID)]
    pub asset: UncheckedAccount<'info>,
    /// CHECK: arbitrary recipient; no data or signer privileges are trusted.
    pub new_owner: UncheckedAccount<'info>,
    #[account(mut, seeds = [ORACLE_SEED, config.key().as_ref()], bump = oracle.bump, has_one = config)]
    pub oracle: Account<'info, TransferOracle>,
    /// CHECK: fixed, executable Core program.
    #[account(address = mpl_core::ID, executable)]
    pub core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> TransferNft<'info> {
    pub fn transfer_nft(&mut self) -> Result<()> {
        require_owned_member(
            &load_asset(&self.asset)?,
            &self.owner.key(),
            &self.collection.key(),
        )?;
        let now = Clock::get()?.unix_timestamp;
        require!(self.oracle.is_open(now)?, StakingError::TransfersClosed);
        self.oracle.refresh(now)?;
        self.oracle.core_validation.transfer = ValidationState::Pass;
        self.oracle.exit(&crate::ID)?;
        TransferV1CpiBuilder::new(&self.core_program)
            .asset(&self.asset)
            .collection(Some(&self.collection))
            .payer(&self.owner)
            .authority(Some(&self.owner))
            .new_owner(&self.new_owner)
            .system_program(Some(&self.system_program))
            .add_remaining_account(&self.oracle.to_account_info(), false, false)
            .invoke()?;
        self.oracle.core_validation.transfer = ValidationState::Rejected;
        self.oracle.exit(&crate::ID)?;
        Ok(())
    }
}
