/// Seed of the per-collection `StakeConfig` PDA. The config PDA is also the
/// authority of every staked asset's FreezeDelegate plugin, so only this
/// program can unfreeze a staked asset.
pub const CONFIG_SEED: &[u8] = b"config";

/// Seed of the per-asset `StakeRecord` PDA.
pub const STAKE_SEED: &[u8] = b"stake";

pub const REWARD_MINT_SEED: &[u8] = b"reward_mint";
pub const ORACLE_SEED: &[u8] = b"oracle";
pub const ORACLE_VAULT_SEED: &[u8] = b"oracle_vault";
pub const REWARD_DECIMALS: u8 = 6;
pub const TOTAL_STAKED_ATTRIBUTE: &str = "total_staked";
pub const SECONDS_PER_DAY: i64 = 86_400;
/// Crank rewards are available for 60 seconds starting at each boundary.
pub const CRANK_WINDOW_SECONDS: i64 = 60;
