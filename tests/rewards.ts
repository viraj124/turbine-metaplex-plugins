import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { BURN_BONUS, MIN_LOCK_SECONDS, REWARD_RATE, TestStaking } from "./helpers/core-staking";
import { expectFailure } from "./helpers/expect-failure";

describe("rewards and collection statistics", () => {
  let staking: TestStaking;
  beforeEach(async () => { staking = new TestStaking(); await staking.create(); });

  it("initializes a PDA mint with six decimals, config authority, and no freeze authority", () => {
    const mint = staking.provider.getAccountInfo(staking.rewardMint)!.data;
    expect(mint.readUInt32LE(0)).to.equal(1);
    expect(mint.subarray(4, 36).equals(staking.config.toBuffer())).to.be.true;
    expect(mint[44]).to.equal(6);
    expect(mint.readUInt32LE(46)).to.equal(0);
    expect(staking.rewardSupply()).to.equal(0n);
    expect(staking.collectionAttributes().total_staked).to.equal("0");
  });

  it("mints to the owner's ATA while preserving the stake, freeze and original lock start", async () => {
    const { owner, asset } = staking.holder();
    await staking.stake(owner, asset);
    const stakedAt = staking.readStakeRecord(asset).stakedAt.toString();
    staking.warp(90n);
    await staking.claim(owner, asset);
    expect(staking.rewardBalance(owner.publicKey)).to.equal(90n * REWARD_RATE);
    expect(staking.rewardSupply()).to.equal(90n * REWARD_RATE);
    expect(staking.readStakeRecord(asset).stakedAt.toString()).to.equal(stakedAt);
    expect(staking.readStakeRecord(asset).lastClaimedAt.toString()).to.equal(staking.now().toString());
    expect(staking.assetOwner(asset).equals(owner.publicKey)).to.be.true;
    expect(staking.collectionAttributes().total_staked).to.equal("1");
    await expectFailure(() => staking.transfer(owner, asset, Keypair.generate().publicKey), "custom program error");
    await expectFailure(() => staking.unstake(owner, asset), "StillLocked");
  });

  it("settles only new seconds across claims and unstaking", async () => {
    const { owner, asset } = staking.holder();
    await staking.stake(owner, asset);
    await expectFailure(() => staking.claim(owner, asset), "NoRewards");
    staking.warp(10n);
    await staking.claim(owner, asset);
    await expectFailure(() => staking.claim(owner, asset), "NoRewards");
    staking.warp(20n);
    await staking.claim(owner, asset);
    expect(staking.rewardBalance(owner.publicKey)).to.equal(30n * REWARD_RATE);
    staking.warp(MIN_LOCK_SECONDS - 30n);
    await staking.unstake(owner, asset);
    expect(staking.rewardBalance(owner.publicKey)).to.equal(MIN_LOCK_SECONDS * REWARD_RATE);
    expect(staking.rewardSupply()).to.equal(MIN_LOCK_SECONDS * REWARD_RATE);
    expect(staking.collectionAttributes().total_staked).to.equal("0");
  });

  it("burns through BurnDelegate, settles pending rewards and pays the bonus exactly once", async () => {
    const { owner, asset } = staking.holder();
    await staking.stake(owner, asset);
    staking.warp(10n);
    await staking.claim(owner, asset);
    staking.warp(MIN_LOCK_SECONDS - 10n);
    await staking.burn(owner, asset);
    expect(staking.rewardBalance(owner.publicKey)).to.equal(MIN_LOCK_SECONDS * REWARD_RATE + BURN_BONUS);
    expect(staking.rewardSupply()).to.equal(MIN_LOCK_SECONDS * REWARD_RATE + BURN_BONUS);
    expect(staking.exists(staking.stakeRecord(asset))).to.be.false;
    const burned = staking.provider.getAccountInfo(asset);
    expect(burned === null || burned.data[0] === 0).to.be.true;
    expect(staking.readConfig().totalStaked.toNumber()).to.equal(0);
    expect(staking.collectionAttributes().total_staked).to.equal("0");
    await expectFailure(() => staking.burn(owner, asset), "AccountNotInitialized");
    expect(staking.rewardSupply()).to.equal(MIN_LOCK_SECONDS * REWARD_RATE + BURN_BONUS);
  });

  it("enforces the burn lock and owner authorization for both payout paths", async () => {
    const { owner, asset } = staking.holder();
    const other = staking.holder().owner;
    await staking.stake(owner, asset);
    staking.warp(1n);
    await expectFailure(() => staking.burn(owner, asset), "StillLocked");
    await expectFailure(() => staking.claim(other, asset), "NotAssetOwner");
    await expectFailure(() => staking.burn(other, asset), "NotAssetOwner");
    expect(staking.rewardSupply()).to.equal(0n);
    expect(staking.collectionAttributes().total_staked).to.equal("1");
  });

  it("maintains the collection count across multiple holders, burn, unstake, and restake", async () => {
    const first = staking.holder(), second = staking.holder();
    await staking.stake(first.owner, first.asset);
    await staking.stake(second.owner, second.asset);
    expect(staking.collectionAttributes().total_staked).to.equal("2");
    staking.warp(MIN_LOCK_SECONDS);
    await staking.burn(first.owner, first.asset);
    expect(staking.collectionAttributes().total_staked).to.equal("1");
    await staking.unstake(second.owner, second.asset);
    expect(staking.collectionAttributes().total_staked).to.equal("0");
    await staking.stake(second.owner, second.asset);
    expect(staking.collectionAttributes().total_staked).to.equal("1");
    expect(staking.readConfig().totalStaked.toNumber()).to.equal(1);
  });

  it("preserves unrelated attributes when initializing and updating staking statistics", async () => {
    const fresh = new TestStaking();
    fresh.createCollection(fresh.collectionKeypair, fresh.admin);
    fresh.addCollectionAttributes({ theme: "space", total_staked: "old" });
    await fresh.initializeConfig(fresh.collection, fresh.admin);
    expect(fresh.collectionAttributes()).to.deep.equal({ theme: "space", total_staked: "0" });
    const { owner, asset } = fresh.holder();
    await fresh.stake(owner, asset);
    expect(fresh.collectionAttributes()).to.deep.equal({ theme: "space", total_staked: "1" });
    fresh.warp(MIN_LOCK_SECONDS);
    await fresh.burn(owner, asset);
    expect(fresh.collectionAttributes()).to.deep.equal({ theme: "space", total_staked: "0" });
  });

  it("rejects invalid economics and reinitialization", async () => {
    const collection = Keypair.generate();
    staking.createCollection(collection, staking.admin);
    await expectFailure(() => staking.initializeConfig(collection.publicKey, staking.admin, 0n, 0n), "InvalidRewards");
    await expectFailure(() => staking.initializeConfig(collection.publicKey, staking.admin, 0n, REWARD_RATE, 0n), "InvalidRewards");
    await expectFailure(() => staking.initializeConfig(staking.collection, staking.admin), "already in use");
  });

  it("starts a fresh accrual checkpoint when restaked", async () => {
    const { owner, asset } = staking.holder();
    await staking.stake(owner, asset);
    staking.warp(MIN_LOCK_SECONDS);
    await staking.unstake(owner, asset);
    staking.warp(1_000n);
    await staking.stake(owner, asset);
    staking.warp(5n);
    await staking.claim(owner, asset);
    expect(staking.rewardBalance(owner.publicKey)).to.equal((MIN_LOCK_SECONDS + 5n) * REWARD_RATE);
  });

  it("fails overflowing payouts atomically without burning or unfreezing the NFT", async () => {
    const fresh = new TestStaking();
    fresh.createCollection(fresh.collectionKeypair, fresh.admin);
    const max = (1n << 64n) - 1n;
    await fresh.initializeConfig(fresh.collection, fresh.admin, 0n, max, 1n);
    const { owner, asset } = fresh.holder();
    await fresh.stake(owner, asset);
    fresh.warp(1n);
    await expectFailure(() => fresh.burn(owner, asset), "ArithmeticOverflow");
    fresh.warp(1n);
    await expectFailure(() => fresh.claim(owner, asset), "ArithmeticOverflow");
    expect(fresh.rewardSupply()).to.equal(0n);
    expect(fresh.assetOwner(asset).equals(owner.publicKey)).to.be.true;
    expect(fresh.exists(fresh.stakeRecord(asset))).to.be.true;
    expect(fresh.collectionAttributes().total_staked).to.equal("1");
    await expectFailure(() => fresh.transfer(owner, asset, Keypair.generate().publicKey), "custom program error");
  });

  it("rejects a reward ATA belonging to a different holder", async () => {
    const { owner, asset } = staking.holder();
    const other = staking.holder().owner;
    await staking.stake(owner, asset);
    staking.warp(1n);
    await expectFailure(() => staking.program.methods.claimRewards().accountsPartial({
      ...staking.rewardAccounts(owner.publicKey, asset), rewardsAta: staking.rewardsAta(other.publicKey),
    }).signers([owner]).rpc(), "failed");
    expect(staking.rewardSupply()).to.equal(0n);
  });

});
