use anchor_lang::prelude::*;

/// One per Core collection that can be staked.
#[account]
#[derive(InitSpace)]
pub struct StakeConfig {
    /// The collection's update authority at the time the config was created.
    pub admin: Pubkey,
    pub collection: Pubkey,
    /// Seconds an asset has to stay staked before it can be unstaked.
    pub min_lock_seconds: i64,
    /// Assets currently staked.
    pub total_staked: u64,
    pub reward_mint: Pubkey,
    /// Immutable base units per second, per NFT (mint has 6 decimals).
    pub reward_rate: u64,
    /// Additional base units paid once on a successful burn.
    pub burn_bonus: u64,
    pub reward_mint_bump: u8,
    pub bump: u8,
}

/// One per staked asset. Created by `stake`, closed by `unstake` or `burn_staked_nft`, so its
/// existence alone means the asset is staked.
#[account]
#[derive(InitSpace)]
pub struct StakeRecord {
    pub owner: Pubkey,
    pub asset: Pubkey,
    pub config: Pubkey,
    pub staked_at: i64,
    pub last_claimed_at: i64,
    pub bump: u8,
}

impl StakeRecord {
    pub fn pending_rewards(&self, now: i64, rate: u64) -> Result<u64> {
        let elapsed = now
            .checked_sub(self.last_claimed_at)
            .ok_or(crate::error::StakingError::ArithmeticOverflow)?;
        let elapsed =
            u64::try_from(elapsed).map_err(|_| crate::error::StakingError::InvalidTimestamp)?;
        elapsed
            .checked_mul(rate)
            .ok_or_else(|| error!(crate::error::StakingError::ArithmeticOverflow))
    }

    pub fn require_unlocked(&self, now: i64, lock: i64) -> Result<()> {
        let elapsed = now
            .checked_sub(self.staked_at)
            .ok_or(crate::error::StakingError::ArithmeticOverflow)?;
        require!(elapsed >= lock, crate::error::StakingError::StillLocked);
        Ok(())
    }
}

/// Wire values match Core's ExternalValidationResult Borsh enum.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ValidationState {
    Approved,
    Rejected,
    Pass,
}

/// Wire-compatible with Core's OracleValidation::V1 (tag then four results).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, InitSpace)]
pub struct LifecycleValidation {
    pub version: u8,
    pub create: ValidationState,
    pub transfer: ValidationState,
    pub burn: ValidationState,
    pub update: ValidationState,
}

impl LifecycleValidation {
    pub fn new(transfer: ValidationState) -> Self {
        Self {
            version: 1,
            create: ValidationState::Pass,
            transfer,
            burn: ValidationState::Pass,
            update: ValidationState::Pass,
        }
    }
}

#[account]
#[derive(InitSpace)]
pub struct TransferOracle {
    /// Core reads this at offset 8. Rejected at rest; Pass only during the
    /// clock-checked transfer CPI. This prevents stale crank data authorizing
    /// out-of-hours direct Core transfers.
    pub core_validation: LifecycleValidation,
    /// Permissionless cranks publish the observed schedule here.
    pub validation: LifecycleValidation,
    pub config: Pubkey,
    pub open_hour: u8,
    pub close_hour: u8,
    pub updated_at: i64,
    pub crank_reward: u64,
    /// -1 means no boundary has been rewarded; timestamps must be nonnegative.
    pub last_rewarded_boundary: i64,
    pub bump: u8,
}

impl TransferOracle {
    pub fn is_open(&self, now: i64) -> Result<bool> {
        require!(now >= 0, crate::error::StakingError::InvalidTimestamp);
        let second = now % crate::SECONDS_PER_DAY;
        Ok(
            second >= i64::from(self.open_hour) * 3600
                && second < i64::from(self.close_hour) * 3600,
        )
    }

    pub fn refresh(&mut self, now: i64) -> Result<()> {
        self.validation = LifecycleValidation::new(if self.is_open(now)? {
            ValidationState::Pass
        } else {
            ValidationState::Rejected
        });
        self.updated_at = now;
        Ok(())
    }

    pub fn reward_boundary(&self, now: i64) -> Result<Option<i64>> {
        require!(now >= 0, crate::error::StakingError::InvalidTimestamp);
        let second = now % crate::SECONDS_PER_DAY;
        // close_hour == 24 is the midnight boundary.
        for hour in [self.open_hour, self.close_hour] {
            let boundary_second = (i64::from(hour) * 3600) % crate::SECONDS_PER_DAY;
            let elapsed = second - boundary_second;
            if (0..crate::CRANK_WINDOW_SECONDS).contains(&elapsed) {
                return Ok(Some(now - elapsed));
            }
        }
        Ok(None)
    }
}

#[account]
#[derive(InitSpace)]
pub struct OracleVault {
    pub config: Pubkey,
    pub bump: u8,
}
