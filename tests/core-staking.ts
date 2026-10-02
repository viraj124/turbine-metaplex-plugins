import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import {
  CORE_INVALID_AUTHORITY,
  CORE_NO_APPROVALS,
  MIN_LOCK_SECONDS,
  TestStaking,
} from "./helpers/core-staking";
import { expectFailure } from "./helpers/expect-failure";

describe("core_staking", () => {
  let staking: TestStaking;

  beforeEach(async () => {
    staking = new TestStaking();
    await staking.create();
  });

  describe("initialize_config", () => {
    it("records the collection, admin and lock period", async () => {
      const config = staking.readConfig();

      expect(config.admin.equals(staking.admin.publicKey)).to.be.true;
      expect(config.collection.equals(staking.collection)).to.be.true;
      expect(config.minLockSeconds.toString()).to.equal(
        MIN_LOCK_SECONDS.toString(),
      );
      expect(config.totalStaked.toNumber()).to.equal(0);
    });

    it("refuses anyone but the collection's update authority", async () => {
      const collection = Keypair.generate();
      staking.createCollection(collection, staking.admin);
      const impostor = Keypair.generate();
      staking.provider.airdrop(impostor.publicKey, 1_000_000_000n);

      await expectFailure(
        () => staking.initializeConfig(collection.publicKey, impostor),
        "NotCollectionAuthority",
      );
    });

    it("refuses a negative lock period", async () => {
      const collection = Keypair.generate();
      staking.createCollection(collection, staking.admin);

      await expectFailure(
        () =>
          staking.initializeConfig(collection.publicKey, staking.admin, -1n),
        "NegativeLockPeriod",
      );
    });
  });

  describe("stake", () => {
    it("records the stake and leaves the asset in the owner's wallet", async () => {
      const { owner, asset } = staking.holder();

      await staking.stake(owner, asset);

      const record = staking.readStakeRecord(asset);
      expect(record.owner.equals(owner.publicKey)).to.be.true;
      expect(record.asset.equals(asset)).to.be.true;
      expect(record.config.equals(staking.config)).to.be.true;
      expect(record.stakedAt.toString()).to.equal(staking.now().toString());
      expect(staking.assetOwner(asset).equals(owner.publicKey), "not moved").to
        .be.true;

      const config = staking.readConfig();
      expect(config.totalStaked.toNumber()).to.equal(1);
    });

    it("freezes the asset so it cannot be transferred", async () => {
      const { owner, asset } = staking.holder();
      await staking.stake(owner, asset);

      await expectFailure(
        () => staking.transfer(owner, asset, Keypair.generate().publicKey),
        CORE_INVALID_AUTHORITY,
      );
    });

    it("stops the owner thawing, revoking or removing the freeze directly", async () => {
      const { owner, asset } = staking.holder();
      await staking.stake(owner, asset);

      await expectFailure(
        () => staking.ownerCoreCall(owner, asset, TestStaking.THAW),
        CORE_NO_APPROVALS,
      );
      await expectFailure(
        () => staking.ownerCoreCall(owner, asset, TestStaking.REVOKE_FREEZE),
        CORE_INVALID_AUTHORITY,
      );
      await expectFailure(
        () => staking.ownerCoreCall(owner, asset, TestStaking.REMOVE_FREEZE),
        CORE_INVALID_AUTHORITY,
      );
      expect(staking.exists(staking.stakeRecord(asset)), "still staked").to.be
        .true;
    });

    it("refuses an asset the signer does not own", async () => {
      const { asset } = staking.holder();
      const thief = Keypair.generate();
      staking.provider.airdrop(thief.publicKey, 1_000_000_000n);

      await expectFailure(() => staking.stake(thief, asset), "NotAssetOwner");
    });

    it("refuses an asset from another collection", async () => {
      const other = Keypair.generate();
      staking.createCollection(other, staking.admin);
      const owner = Keypair.generate();
      staking.provider.airdrop(owner.publicKey, 1_000_000_000n);
      const asset = staking.mintAsset(owner.publicKey, other.publicKey);

      await expectFailure(() => staking.stake(owner, asset), "WrongCollection");
    });

    it("refuses an asset that is already staked", async () => {
      const { owner, asset } = staking.holder();
      await staking.stake(owner, asset);

      await expectFailure(() => staking.stake(owner, asset), "already in use");
    });
  });

  describe("unstake", () => {
    it("refuses to unstake inside the lock period", async () => {
      const { owner, asset } = staking.holder();
      await staking.stake(owner, asset);
      staking.warp(MIN_LOCK_SECONDS - 1n);

      await expectFailure(() => staking.unstake(owner, asset), "StillLocked");
    });

    it("thaws the asset, closes the record and refunds its rent", async () => {
      const { owner, asset } = staking.holder();
      await staking.stake(owner, asset);
      staking.warp(MIN_LOCK_SECONDS);
      const before = staking.provider.getAccountInfo(owner.publicKey)!.lamports;
      const recordRent = staking.provider.getAccountInfo(
        staking.stakeRecord(asset),
      )!.lamports;

      await staking.unstake(owner, asset);

      expect(staking.exists(staking.stakeRecord(asset)), "record closed").to.be
        .false;
      expect(
        staking.provider.getAccountInfo(owner.publicKey)!.lamports,
      ).to.be.greaterThanOrEqual(before + recordRent - 2_100_000);
      const config = staking.readConfig();
      expect(config.totalStaked.toNumber()).to.equal(0);

      const recipient = Keypair.generate().publicKey;
      staking.transfer(owner, asset, recipient);
      expect(staking.assetOwner(asset).equals(recipient)).to.be.true;
    });

    it("lets the asset be staked again", async () => {
      const { owner, asset } = staking.holder();
      await staking.stake(owner, asset);
      staking.warp(MIN_LOCK_SECONDS);
      await staking.unstake(owner, asset);

      await staking.stake(owner, asset);

      expect(staking.exists(staking.stakeRecord(asset))).to.be.true;
    });

    it("refuses anyone but the staker", async () => {
      const { owner, asset } = staking.holder();
      await staking.stake(owner, asset);
      staking.warp(MIN_LOCK_SECONDS);
      const thief = Keypair.generate();
      staking.provider.airdrop(thief.publicKey, 1_000_000_000n);

      await expectFailure(() => staking.unstake(thief, asset), "NotAssetOwner");
      expect(staking.exists(staking.stakeRecord(asset)), "still staked").to.be
        .true;
    });

    it("refuses an asset that was never staked", async () => {
      const { owner, asset } = staking.holder();

      await expectFailure(
        () => staking.unstake(owner, asset),
        "AccountNotInitialized",
      );
    });
  });
});
