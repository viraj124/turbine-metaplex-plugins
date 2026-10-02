import { Keypair } from "@solana/web3.js";
import { expect } from "chai";
import { CRANK_REWARD, MIN_LOCK_SECONDS, TestStaking } from "./helpers/core-staking";
import { expectFailure } from "./helpers/expect-failure";
const DAY = 86_400n;
const OPEN = 9n * 3_600n;
const CLOSE = 17n * 3_600n;

describe("collection transfer oracle", () => {
  let staking: TestStaking;
  beforeEach(async () => {
    staking = new TestStaking(); await staking.create();
    staking.setTime(DAY + OPEN - 1n);
    await staking.initializeOracle();
  });

  it("stores Core's V1 lifecycle format after the Anchor discriminator", () => {
    const data = staking.provider.getAccountInfo(staking.oracle)!.data;
    expect([...data.subarray(8, 13)]).to.deep.equal([1, 2, 1, 2, 2]);
    expect(staking.readOracle().validation.transfer).to.deep.equal({ rejected: {} });
  });

  it("transfers at opening and before closing, but rejects either side of the window", async () => {
    const { owner, asset } = staking.holder();
    const recipient = Keypair.generate();
    staking.provider.airdrop(recipient.publicKey, 1_000_000_000n);
    await expectFailure(() => staking.transferNft(owner, asset, recipient.publicKey), "TransfersClosed");
    staking.setTime(DAY + OPEN);
    await staking.transferNft(owner, asset, recipient.publicKey);
    expect(staking.assetOwner(asset).equals(recipient.publicKey)).to.be.true;
    expect(staking.readOracle().coreValidation.transfer).to.deep.equal({ rejected: {} });
    staking.setTime(DAY + CLOSE - 1n);
    await staking.transferNft(recipient, asset, owner.publicKey);
    staking.setTime(DAY + CLOSE);
    await expectFailure(() => staking.transferNft(owner, asset, recipient.publicKey), "TransfersClosed");
  });

  it("keeps the direct Core gate closed even when the last crank observed open hours", async () => {
    const { owner, asset } = staking.holder();
    staking.setTime(DAY + OPEN);
    await staking.updateOracle();
    expect(staking.readOracle().validation.transfer).to.deep.equal({ pass: {} });
    await expectFailure(() => staking.transfer(owner, asset, Keypair.generate().publicKey, true), "custom program error");
    staking.setTime(DAY + CLOSE);
    await expectFailure(() => staking.transfer(owner, asset, Keypair.generate().publicKey, true), "custom program error");
    await expectFailure(() => staking.transferNft(owner, asset, Keypair.generate().publicKey), "TransfersClosed");
    await staking.updateOracle();
    expect(staking.readOracle().validation.transfer).to.deep.equal({ rejected: {} });
  });

  it("keeps staked NFTs frozen during open hours and restores the gate after failed transfers", async () => {
    const { owner, asset } = staking.holder();
    await staking.stake(owner, asset);
    staking.setTime(DAY + OPEN);
    await expectFailure(() => staking.transferNft(owner, asset, Keypair.generate().publicKey), "custom program error");
    expect(staking.readOracle().coreValidation.transfer).to.deep.equal({ rejected: {} });
    expect(staking.assetOwner(asset).equals(owner.publicKey)).to.be.true;
    staking.warp(MIN_LOCK_SECONDS);
    await staking.unstake(owner, asset);
    await staking.transferNft(owner, asset, Keypair.generate().publicKey);
  });

  it("pays any caller once per boundary, only during the 60-second window", async () => {
    await staking.fundOracle(10n * CRANK_REWARD);
    const caller = staking.holder().owner;
    const balance = () => staking.provider.getAccountInfo(caller.publicKey)!.lamports;
    const before = balance();
    await staking.updateOracle(caller);
    expect(balance()).to.equal(before);
    staking.setTime(DAY + OPEN);
    await staking.updateOracle(caller);
    expect(balance()).to.equal(before + Number(CRANK_REWARD));
    await staking.updateOracle(caller);
    expect(balance()).to.equal(before + Number(CRANK_REWARD));
    staking.setTime(DAY + CLOSE + 59n);
    await staking.updateOracle(caller);
    expect(balance()).to.equal(before + 2 * Number(CRANK_REWARD));
    staking.setTime(2n * DAY + OPEN + 60n);
    await staking.updateOracle(caller);
    expect(balance()).to.equal(before + 2 * Number(CRANK_REWARD));
    staking.setTime(2n * DAY + CLOSE);
    await staking.updateOracle(caller);
    expect(balance()).to.equal(before + 3 * Number(CRANK_REWARD));
  });

  it("updates with an empty vault, preserves rent and can pay after funding within the window", async () => {
    const rent = staking.provider.getAccountInfo(staking.oracleVault)!.lamports;
    staking.setTime(DAY + OPEN);
    await staking.updateOracle();
    expect(staking.readOracle().validation.transfer).to.deep.equal({ pass: {} });
    expect(staking.readOracle().lastRewardedBoundary.toString()).to.equal("-1");
    await staking.fundOracle(CRANK_REWARD);
    await staking.updateOracle();
    expect(staking.provider.getAccountInfo(staking.oracleVault)!.lamports).to.equal(rent);
    staking.setTime(DAY + CLOSE);
    await staking.updateOracle();
    expect(staking.readOracle().validation.transfer).to.deep.equal({ rejected: {} });
  });

  it("allows the current collection authority to recover only the spendable vault budget", async () => {
    const rent = staking.provider.getAccountInfo(staking.oracleVault)!.lamports;
    await staking.fundOracle(2n * CRANK_REWARD);
    await expectFailure(() => staking.withdrawOracle(CRANK_REWARD, staking.holder().owner), "NotCollectionAuthority");
    await expectFailure(() => staking.withdrawOracle(2n * CRANK_REWARD + 1n), "InsufficientVaultFunds");
    await staking.withdrawOracle(2n * CRANK_REWARD);
    expect(staking.provider.getAccountInfo(staking.oracleVault)!.lamports).to.equal(rent);
  });

  it("rejects wrong-owner transfers and duplicate oracle initialization", async () => {
    const holder = staking.holder();
    staking.setTime(DAY + OPEN);
    await expectFailure(() => staking.transferNft(staking.holder().owner, holder.asset, Keypair.generate().publicKey), "NotAssetOwner");
    await expectFailure(() => staking.initializeOracle(), "already in use");
  });

  it("validates schedule and initialization authority", async () => {
    const fresh = new TestStaking(); await fresh.create();
    await expectFailure(() => fresh.initializeOracle(17, 9), "InvalidTransferHours");
    await expectFailure(() => fresh.initializeOracle(9, 9), "InvalidTransferHours");
    await expectFailure(() => fresh.initializeOracle(9, 25), "InvalidTransferHours");
    await expectFailure(() => fresh.initializeOracle(9, 17, 0n), "InvalidAmount");
    await expectFailure(() => fresh.initializeOracle(9, 17, CRANK_REWARD, fresh.holder().owner), "NotCollectionAuthority");
  });

  it("handles a closing boundary at midnight", async () => {
    const fresh = new TestStaking(); await fresh.create();
    fresh.setTime(DAY + 23n * 3_600n);
    await fresh.initializeOracle(9, 24);
    await fresh.fundOracle(CRANK_REWARD);
    const { owner, asset } = fresh.holder();
    await fresh.transferNft(owner, asset, owner.publicKey);
    fresh.setTime(2n * DAY);
    await fresh.updateOracle(owner);
    expect(fresh.readOracle().validation.transfer).to.deep.equal({ rejected: {} });
    expect(fresh.readOracle().lastRewardedBoundary.toString()).to.equal((2n * DAY).toString());
    await expectFailure(() => fresh.transferNft(owner, asset, Keypair.generate().publicKey), "TransfersClosed");
  });

});
