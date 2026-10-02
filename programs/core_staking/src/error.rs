use anchor_lang::error_code;

#[error_code]
pub enum StakingError {
    #[msg("account is not a Core asset")]
    NotAnAsset,
    #[msg("account is not a Core collection")]
    NotACollection,
    #[msg("signer is not the collection's update authority")]
    NotCollectionAuthority,
    #[msg("signer does not own the asset")]
    NotAssetOwner,
    #[msg("asset does not belong to the staking collection")]
    WrongCollection,
    #[msg("asset already has a freeze delegate, revoke it before staking")]
    AlreadyFreezeDelegated,
    #[msg("minimum lock period cannot be negative")]
    NegativeLockPeriod,
    #[msg("asset is still inside its minimum lock period")]
    StillLocked,
    #[msg("staked count overflowed")]
    StakedCountOverflow,
    #[msg("asset already has a burn delegate; remove it before staking")]
    AlreadyBurnDelegated,
    #[msg("reward rate and burn bonus must be positive")]
    InvalidRewards,
    #[msg("arithmetic overflow")]
    ArithmeticOverflow,
    #[msg("on-chain timestamp is invalid or went backwards")]
    InvalidTimestamp,
    #[msg("no rewards have accrued since the last claim")]
    NoRewards,
    #[msg("collection attributes are missing, inconsistent, or not delegated to the config")]
    InvalidCollectionStats,
    #[msg("asset is not frozen by this staking config")]
    InvalidFreezeDelegate,
    #[msg("hours must satisfy 0 <= open < close <= 24")]
    InvalidTransferHours,
    #[msg("NFT transfers are currently outside the allowed UTC hours")]
    TransfersClosed,
    #[msg("amount must be positive")]
    InvalidAmount,
    #[msg("vault has insufficient spendable lamports")]
    InsufficientVaultFunds,
}
