# Security notes

Risk level: **Critical** — the program signs Core plugin CPIs, mints tokens, permanently burns owner-authorized NFTs, and holds a lamport crank budget.

Design guidance: [Safe Solana Builder](https://github.com/Frankcastleauditor/safe-solana-builder/tree/9e94436dcf4b5dfd6d837eb70cf88b9048e72e5d). This is an implementation checklist, not an independent audit.

## Applied checks

- [x] Collection initialization verifies the current Core collection update authority, rather than granting privileges to the first caller.
- [x] Core accounts are owner-constrained and their AssetV1/CollectionV1 layouts are validated where read. Asset ownership and collection membership are checked for every NFT action.
- [x] Config, stake record, mint, oracle and vault have distinct canonical PDA seeds. Anchor enforces stored bumps and cross-account relationships.
- [x] Core is address-constrained and executable. System, SPL Token and ATA program accounts are typed and pinned. No arbitrary caller-supplied CPI programs are accepted.
- [x] Claim, unstake and burn validate the stake-record owner, asset, config, and actual config-controlled frozen FreezeDelegate.
- [x] Burn is explicitly authorized by the staker and uses the config's BurnDelegate CPI authority. No admin burn entrypoint exists.
- [x] Mint and destination are constrained to the config's mint and the staker's canonical ATA. The six-decimal legacy mint has no freeze authority. Only config signs minting.
- [x] `init_if_needed` is used only for canonical ATAs with owner and mint constraints. Program state uses one-time `init`.
- [x] All payouts share the same reward calculation and mint helper. Every payout advances the checkpoint or closes the record. Claims do not reset the original lock time.
- [x] Both exit paths honor the same minimum lock. Claims deliberately remain available during the lock, as required.
- [x] Reward multiplication, bonus addition and stake counters use checked arithmetic. Backward claim timestamps return an error.
- [x] Collection count updates preserve unrelated attributes and verify the old count and authority. Core data is re-read after preceding Core CPIs; no cached collection data is reused.
- [x] NFT changes, rewards, collection statistics and stake-record closure are atomic. CPI errors are propagated.
- [x] Anchor closes the stake record to its recorded owner on either exit. Core handles NFT burn/tombstone semantics separately.
- [x] Oracle has a static PDA address and Core-compatible V1 bytes at offset 8. Only Transfer is registered, with REJECT capability (`4`).
- [x] `transfer_nft` reads `Clock` directly and explicitly flushes the temporary oracle result before CPI. The oracle returns to Rejected at instruction completion; failed transfers roll back.
- [x] The only forwarded remaining account is the typed, seed-checked oracle PDA, passed read-only to Core. Caller-provided remaining accounts are not forwarded.
- [x] Permissionless crank rewards use a 60-second post-boundary window and a monotonically increasing last-paid boundary. They cannot consume the rent reserve.
- [x] Empty vaults do not prevent oracle updates or transfers. No accrued or deferred crank liabilities are recorded.
- [x] Vault funding requires the source signer. Withdrawal checks the current collection update authority and limits payout to spendable lamports.
- [x] No unsafe Rust or user-input `unwrap`/`expect` is used in program handlers.

## High-risk decisions and trust assumptions

**Collection authority.** Core's current collection authority remains trusted. It can change collection governance or plugin configuration through Core. Removing/redelegating Attributes can interrupt staking exits and statistics; removing the oracle adapter can remove transfer restrictions. This program does not take over collection governance. `config.admin` records the setup signer for provenance; later oracle setup and vault withdrawal follow the current on-chain collection authority. The staking program does not add a separate admin-rotation mechanism.

**Program and dependency upgrades.** Deployment upgrade authority is controlled by the deployment environment and is not changed here. Whoever holds it can replace this program. Core and SPL program behavior are trusted. A production deployment should select and document its upgrade/governance policy separately.

**Burn and issuance economics.** A successful burn is irreversible. Both rate and bonus are positive, immutable, admin-selected values; there is no issuance cap or token value guarantee. Extreme rates, elapsed times, or exhausted SPL u64 supply can make reward-bearing operations fail. The program fails atomically rather than wrapping amounts. Collection operators must choose viable economics before admitting stakes.

**Oracle client integration.** The Core-facing result is always Rejected at rest. Clients must use `transfer_nft`, including during open hours. The separate schedule result is observability state, not direct Core authorization. This prevents stale open observations from relaxing the UTC restriction. Wallet/marketplace support must be implemented by the client.

**Vault donations.** The current collection authority can recover unspent budget, and the original depositor has no separate claim to donations. Crank rewards are paid only when funded; they are not guaranteed liabilities. The oracle/vault and config are permanent setup accounts; their rent reserves are retained.

**Plugin combinations.** Tests cover ordinary uncompressed Core collection assets plus this program's plugins. Existing asset FreezeDelegate/BurnDelegate plugins must be removed before staking. Arbitrary royalties, additional oracles/hooks, compressible assets, groups and other third-party plugins are not integrated here; such combinations may reject CPIs or require additional accounts. Core collection governance remains trusted to keep the configured staking plugins usable.

**State migration.** Existing deployed state from the original program cannot be deserialized as the new config/record layout. No migration instruction is included. Do not replace a live deployment without a separately designed migration or legacy exit plan.

## Verification scope

The TypeScript LiteSVM suite executes real SBF programs. It covers staking ownership and lock rules, frozen transfers, claim/exit accounting, BurnDelegate burn and single payout, actual collection Attributes, duplicate initialization, invalid economics, arithmetic failure rollback, ATA constraints, oracle boundaries including midnight, missed crank behavior, empty/refilled vaults, once-per-boundary rewards and rent-preserving authorized withdrawals.

The Core fixture is the pre-existing `tests/fixtures/mpl_core.so`, identified by SHA-256 `96fa631a61234766afa538437c5166c628100dbc70e5c3ddb2f306d5dc5a8ba5`. Its original helper describes it as a mainnet dump; no deployment slot was recorded in the original project. Tests do not certify later Core upgrades, live RPC/wallet behavior, or arbitrary plugin combinations.
