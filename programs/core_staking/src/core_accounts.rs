use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    types::{Key as CoreKey, UpdateAuthority},
};

use crate::error::StakingError;

/// Reads a Core asset. The caller must already have checked that the account
/// is owned by the Core program, since the key byte is what tells an asset
/// apart from a collection.
pub fn load_asset(account: &AccountInfo) -> Result<BaseAssetV1> {
    let asset = BaseAssetV1::from_bytes(&account.try_borrow_data()?)
        .map_err(|_| error!(StakingError::NotAnAsset))?;
    require!(asset.key == CoreKey::AssetV1, StakingError::NotAnAsset);
    Ok(asset)
}

/// Reads a Core collection. Same ownership precondition as `load_asset`.
pub fn load_collection(account: &AccountInfo) -> Result<BaseCollectionV1> {
    let collection = BaseCollectionV1::from_bytes(&account.try_borrow_data()?)
        .map_err(|_| error!(StakingError::NotACollection))?;
    require!(
        collection.key == CoreKey::CollectionV1,
        StakingError::NotACollection
    );
    Ok(collection)
}

/// Checks that `owner` holds the asset and that it sits in `collection`.
pub fn require_owned_member(
    asset: &BaseAssetV1,
    owner: &Pubkey,
    collection: &Pubkey,
) -> Result<()> {
    require_keys_eq!(asset.owner, *owner, StakingError::NotAssetOwner);
    require!(
        asset.update_authority == UpdateAuthority::Collection(*collection),
        StakingError::WrongCollection
    );
    Ok(())
}

/// Verify the actual Core lock as well as the program's stake record.
pub fn require_staking_freeze(asset: &AccountInfo, config: Pubkey) -> Result<()> {
    let (authority, freeze, _) = mpl_core::fetch_asset_plugin::<mpl_core::types::FreezeDelegate>(
        asset,
        mpl_core::types::PluginType::FreezeDelegate,
    )
    .map_err(|_| error!(StakingError::InvalidFreezeDelegate))?;
    require!(
        freeze.frozen && authority == mpl_core::types::PluginAuthority::Address { address: config },
        StakingError::InvalidFreezeDelegate
    );
    Ok(())
}

/// Re-read Attributes after prior Core CPIs; preserve every unrelated attribute.
pub fn update_collection_count<'info>(
    core: &AccountInfo<'info>,
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    config: &Account<'info, crate::StakeConfig>,
    system: &AccountInfo<'info>,
    new_count: u64,
) -> Result<()> {
    use mpl_core::{
        fetch_collection_plugin,
        instructions::UpdateCollectionPluginV1CpiBuilder,
        types::{Attributes, Plugin, PluginAuthority, PluginType},
    };
    let (authority, mut attrs, _) =
        fetch_collection_plugin::<Attributes>(collection, PluginType::Attributes)
            .map_err(|_| error!(StakingError::InvalidCollectionStats))?;
    require!(
        authority
            == PluginAuthority::Address {
                address: config.key()
            },
        StakingError::InvalidCollectionStats
    );
    let mut counters = attrs
        .attribute_list
        .iter_mut()
        .filter(|a| a.key == crate::TOTAL_STAKED_ATTRIBUTE);
    let counter = counters
        .next()
        .ok_or(StakingError::InvalidCollectionStats)?;
    require!(
        counter.value == config.total_staked.to_string(),
        StakingError::InvalidCollectionStats
    );
    counter.value = new_count.to_string();
    require!(
        counters.next().is_none(),
        StakingError::InvalidCollectionStats
    );
    UpdateCollectionPluginV1CpiBuilder::new(core)
        .collection(collection)
        .payer(payer)
        .authority(Some(&config.to_account_info()))
        .system_program(system)
        .plugin(Plugin::Attributes(attrs))
        .invoke_signed(&[&[
            crate::CONFIG_SEED,
            config.collection.as_ref(),
            &[config.bump],
        ]])?;
    Ok(())
}
