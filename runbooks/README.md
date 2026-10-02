# Build and rollout

This project uses the same Anchor instruction/state/test separation as the AMM reference. These steps describe operations; no network deployment is performed by the test suite.

## Local verification

```sh
bun install --frozen-lockfile
bun run test
cargo fmt --all -- --check
cargo clippy --offline --all-targets -- -D warnings
```

Keep Anchor CLI 1.2.0 and Solana CLI 3.1.10 consistent with `Anchor.toml`. The first build can download Solana platform tools. The original `target/deploy/core_staking-keypair.json` was retained; no private key belongs in source control.

## Fresh deployment

1. Select the intended cluster, program identity and deployment authority. Ensure `declare_id!`, `Anchor.toml` and the deployment keypair all agree. Build again if changing the program ID.
2. Deploy the compiled program using the chosen Anchor/Solana environment. The configured default is localnet; do not assume this repository points at devnet or mainnet.
3. Create or select a Metaplex Core collection whose current update authority can sign setup. Existing Attributes must be under that authority's control.
4. Run `initialize_config(min_lock_seconds, reward_rate, burn_bonus)` once. Verify the reward mint authority/decimals, collection binding and the Attributes counter before admitting stakes.
5. If desired, call `initialize_oracle(9, 17, crank_reward_lamports)` and fund its vault. Update clients to use `transfer_nft` before enabling this feature.
6. Submit permissionless `update_oracle` transactions shortly after 09:00 and 17:00 UTC. Only the first funded crank in each 60-second window is paid. Scheduling a crank is an operational choice; this repository does not install a background job.

See the root README and integration helpers for account derivations and method examples. Exercise the full flow against the intended deployed Core version before admitting production NFTs.

## Existing deployments

The original config and stake-record layouts and the `initialize_config` ABI changed. Existing PDAs cannot be safely reused by simply upgrading the binary or calling initialization again.

Before replacing an old deployment, either design and validate a migration that preserves its record ownership, original lock times and freeze authorities, or let users exit through the original program and use a fresh program deployment/config namespace. Completing old exits alone does not resize the old config PDA. A new program ID changes all PDA authorities, so it cannot automatically thaw stakes belonging to the old program.

No network deployment, live account migration or transfer of upgrade authority is included in this change.
