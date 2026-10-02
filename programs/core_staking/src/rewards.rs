use crate::{StakeConfig, CONFIG_SEED};
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, MintTo};

/// All payout paths share one mint authority and the same accounting units.
pub fn mint_rewards<'info>(
    config: &Account<'info, StakeConfig>,
    mint: AccountInfo<'info>,
    destination: AccountInfo<'info>,
    token_program: Pubkey,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    mint_to(
        CpiContext::new_with_signer(
            token_program,
            MintTo {
                mint,
                to: destination,
                authority: config.to_account_info(),
            },
            &[&[CONFIG_SEED, config.collection.as_ref(), &[config.bump]]],
        ),
        amount,
    )
}
