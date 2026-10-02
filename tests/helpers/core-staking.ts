import { BN, Program } from "@anchor-lang/core";
import {
  Keypair,
  PublicKey,
  Signer,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import * as path from "path";
import idl from "../../target/idl/core_staking.json";
import { CoreStaking } from "../../target/types/core_staking";
import { LiteSVMProvider } from "./litesvm-provider";

export const PROGRAM_ID = new PublicKey(idl.address);
export const MPL_CORE_ID = new PublicKey(
  "CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d",
);
export const TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
export const ATA_PROGRAM_ID = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
export const REWARD_RATE = 1_000n;
export const BURN_BONUS = 1_000_000_000n;
export const CRANK_REWARD = 10_000n;
export const MIN_LOCK_SECONDS = 7n * 24n * 60n * 60n;

const PROGRAM_PATH = path.join(
  __dirname,
  "../../target/deploy/core_staking.so",
);
const MPL_CORE_PATH = path.join(__dirname, "../fixtures/mpl_core.so");

const CREATE_V1 = 0;
const CREATE_COLLECTION_V1 = 1;
const REMOVE_PLUGIN_V1 = 4;
const UPDATE_PLUGIN_V1 = 6;
const REVOKE_PLUGIN_AUTHORITY_V1 = 10;
const TRANSFER_V1 = 14;
const FREEZE_DELEGATE = 1;

export const CORE_INVALID_AUTHORITY = "custom program error: 0x9";
export const CORE_NO_APPROVALS = "custom program error: 0x1a";

function borshString(value: string): Buffer {
  const bytes = Buffer.from(value, "utf8");
  const length = Buffer.alloc(4);
  length.writeUInt32LE(bytes.length);
  return Buffer.concat([length, bytes]);
}

function meta(pubkey: PublicKey, isWritable: boolean, isSigner = false) {
  return { pubkey, isSigner, isWritable };
}

export class TestStaking {
  readonly provider = new LiteSVMProvider();
  readonly program = new Program<CoreStaking>(
    idl as CoreStaking,
    this.provider,
  );
  readonly admin = Keypair.generate();
  readonly collectionKeypair = Keypair.generate();

  get collection(): PublicKey {
    return this.collectionKeypair.publicKey;
  }

  get config(): PublicKey {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("config"), this.collection.toBuffer()],
      PROGRAM_ID,
    )[0];
  }

  constructor() {
    this.provider.addProgram(PROGRAM_ID, PROGRAM_PATH);
    this.provider.addProgram(MPL_CORE_ID, MPL_CORE_PATH);
    this.provider.airdrop(this.admin.publicKey, 1_000_000_000_000n);
  }

  stakeRecord(asset: PublicKey): PublicKey {
    return PublicKey.findProgramAddressSync(
      [Buffer.from("stake"), asset.toBuffer()],
      PROGRAM_ID,
    )[0];
  }

  async create(minLockSeconds = MIN_LOCK_SECONDS): Promise<void> {
    this.createCollection(this.collectionKeypair, this.admin);
    await this.initializeConfig(this.collection, this.admin, minLockSeconds);
  }

  createCollection(collection: Keypair, updateAuthority: Keypair): void {
    this.send(
      [
        new TransactionInstruction({
          programId: MPL_CORE_ID,
          keys: [
            meta(collection.publicKey, true, true),
            meta(updateAuthority.publicKey, false),
            meta(this.provider.publicKey, true, true),
            meta(SystemProgram.programId, false),
          ],
          data: Buffer.concat([
            Buffer.from([CREATE_COLLECTION_V1]),
            borshString("Treasury Positions"),
            borshString("https://example.com/collection.json"),
            Buffer.from([0]),
          ]),
        }),
      ],
      [collection],
    );
  }

  async initializeConfig(
    collection: PublicKey,
    admin: Keypair,
    minLockSeconds = MIN_LOCK_SECONDS,
    rewardRate = REWARD_RATE,
    burnBonus = BURN_BONUS,
  ): Promise<void> {
    await this.program.methods
      .initializeConfig(new BN(minLockSeconds.toString()), new BN(rewardRate.toString()), new BN(burnBonus.toString()))
      .accounts({ admin: admin.publicKey, collection })
      .signers([admin])
      .rpc();
  }

  mintAsset(
    owner: PublicKey,
    collection = this.collection,
    collectionAuthority = this.admin,
  ): PublicKey {
    const asset = Keypair.generate();
    this.send(
      [
        new TransactionInstruction({
          programId: MPL_CORE_ID,
          keys: [
            meta(asset.publicKey, true, true),
            meta(collection, true),
            meta(collectionAuthority.publicKey, false, true),
            meta(this.provider.publicKey, true, true),
            meta(owner, false),
            meta(MPL_CORE_ID, false),
            meta(SystemProgram.programId, false),
            meta(MPL_CORE_ID, false),
          ],
          data: Buffer.concat([
            Buffer.from([CREATE_V1, 0]),
            borshString("T-Bill Position"),
            borshString("https://example.com/asset.json"),
            Buffer.from([0]),
          ]),
        }),
      ],
      [asset, collectionAuthority],
    );
    return asset.publicKey;
  }

  get rewardMint(): PublicKey { return this.pda("reward_mint"); }
  get oracle(): PublicKey { return this.pda("oracle"); }
  get oracleVault(): PublicKey { return this.pda("oracle_vault"); }

  pda(seed: string): PublicKey {
    return PublicKey.findProgramAddressSync([Buffer.from(seed), this.config.toBuffer()], PROGRAM_ID)[0];
  }

  rewardsAta(owner: PublicKey): PublicKey {
    return PublicKey.findProgramAddressSync([owner.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(),
      this.rewardMint.toBuffer()], ATA_PROGRAM_ID)[0];
  }

  rewardBalance(owner: PublicKey): bigint {
    return this.provider.getAccountInfo(this.rewardsAta(owner))?.data.readBigUInt64LE(64) ?? 0n;
  }

  rewardSupply(): bigint {
    return this.provider.getAccountInfo(this.rewardMint)!.data.readBigUInt64LE(36);
  }

  rewardAccounts(owner: PublicKey, asset: PublicKey) {
    return { owner, config: this.config, collection: this.collection, asset,
      stakeRecord: this.stakeRecord(asset), rewardMint: this.rewardMint, rewardsAta: this.rewardsAta(owner) };
  }

  async claim(owner: Keypair, asset: PublicKey): Promise<void> {
    await this.program.methods.claimRewards().accountsPartial(this.rewardAccounts(owner.publicKey, asset))
      .signers([owner]).rpc();
  }

  async burn(owner: Keypair, asset: PublicKey): Promise<void> {
    await this.program.methods.burnStakedNft().accountsPartial(this.rewardAccounts(owner.publicKey, asset))
      .signers([owner]).rpc();
  }

  async initializeOracle(open = 9, close = 17, reward = CRANK_REWARD, admin = this.admin): Promise<void> {
    await this.program.methods.initializeOracle(open, close, new BN(reward.toString()))
      .accountsPartial({ admin: admin.publicKey, config: this.config, collection: this.collection })
      .signers([admin]).rpc();
  }

  async fundOracle(amount: bigint, funder = this.admin): Promise<void> {
    await this.program.methods.fundOracleVault(new BN(amount.toString()))
      .accountsPartial({ funder: funder.publicKey, config: this.config, oracleVault: this.oracleVault })
      .signers([funder]).rpc();
  }

  async withdrawOracle(amount: bigint, admin = this.admin): Promise<void> {
    await this.program.methods.withdrawOracleVault(new BN(amount.toString()))
      .accountsPartial({ admin: admin.publicKey, config: this.config, collection: this.collection,
        oracleVault: this.oracleVault }).signers([admin]).rpc();
  }

  async updateOracle(caller = this.admin): Promise<void> {
    await this.program.methods.updateOracle().accountsPartial({ caller: caller.publicKey,
      config: this.config, oracle: this.oracle, oracleVault: this.oracleVault }).signers([caller]).rpc();
  }

  async transferNft(owner: Keypair, asset: PublicKey, newOwner: PublicKey): Promise<void> {
    await this.program.methods.transferNft().accountsPartial({ owner: owner.publicKey,
      asset, newOwner, config: this.config, collection: this.collection, oracle: this.oracle })
      .signers([owner]).rpc();
  }

  readOracle() {
    return this.program.coder.accounts.decode("transferOracle", this.provider.getAccountInfo(this.oracle)!.data);
  }

  setTime(seconds: bigint): void {
    const clock = this.provider.svm.getClock();
    clock.unixTimestamp = seconds;
    this.provider.svm.setClock(clock);
  }

  collectionAttributes(): Record<string, string> {
    const data = this.provider.getAccountInfo(this.collection)!.data;
    let cursor = 33;
    const string = (): string => {
      const size = data.readUInt32LE(cursor); cursor += 4;
      const value = data.subarray(cursor, cursor + size).toString(); cursor += size;
      return value;
    };
    string(); string(); cursor += 8;
    cursor = Number(data.readBigUInt64LE(cursor + 1)) + 1;
    const count = data.readUInt32LE(cursor); cursor += 4;
    for (let i = 0; i < count; i++) {
      const type = data[cursor++];
      const authority = data[cursor++];
      if (authority === 3) cursor += 32;
      const offset = Number(data.readBigUInt64LE(cursor)); cursor += 8;
      if (type !== 6) continue;
      cursor = offset + 1;
      const length = data.readUInt32LE(cursor); cursor += 4;
      const attributes: Record<string, string> = {};
      for (let j = 0; j < length; j++) { const key = string(); attributes[key] = string(); }
      return attributes;
    }
    throw new Error("Collection has no Attributes plugin");
  }

  addCollectionAttributes(attributes: Record<string, string>): void {
    const entries = Object.entries(attributes);
    const count = Buffer.alloc(4); count.writeUInt32LE(entries.length);
    this.send([new TransactionInstruction({ programId: MPL_CORE_ID,
      keys: [meta(this.collection, true), meta(this.admin.publicKey, true, true),
        meta(this.admin.publicKey, false, true), meta(SystemProgram.programId, false), meta(MPL_CORE_ID, false)],
      data: Buffer.concat([Buffer.from([3, 6]), count,
        ...entries.flatMap(([key, value]) => [borshString(key), borshString(value)]), Buffer.from([0])]),
    })], [this.admin]);
  }

  holder(): { owner: Keypair; asset: PublicKey } {
    const owner = Keypair.generate();
    this.provider.airdrop(owner.publicKey, 10_000_000_000n);
    return { owner, asset: this.mintAsset(owner.publicKey) };
  }

  async stake(owner: Keypair, asset: PublicKey): Promise<void> {
    await this.program.methods
      .stake()
      .accountsPartial({
        owner: owner.publicKey,
        config: this.config,
        collection: this.collection,
        asset,
      })
      .signers([owner])
      .rpc();
  }

  async unstake(owner: Keypair, asset: PublicKey): Promise<void> {
    await this.program.methods
      .unstake()
      .accountsPartial(this.rewardAccounts(owner.publicKey, asset))
      .signers([owner])
      .rpc();
  }

  transfer(owner: Keypair, asset: PublicKey, newOwner: PublicKey, withOracle = false): void {
    this.send(
      [
        new TransactionInstruction({
          programId: MPL_CORE_ID,
          keys: [
            meta(asset, true),
            meta(this.collection, false),
            meta(this.provider.publicKey, true, true),
            meta(owner.publicKey, false, true),
            meta(newOwner, false),
            meta(SystemProgram.programId, false),
            meta(MPL_CORE_ID, false),
            ...(withOracle ? [meta(this.oracle, false)] : []),
          ],
          data: Buffer.from([TRANSFER_V1, 0]),
        }),
      ],
      [owner],
    );
  }

  ownerCoreCall(owner: Keypair, asset: PublicKey, data: Buffer): void {
    this.send(
      [
        new TransactionInstruction({
          programId: MPL_CORE_ID,
          keys: [
            meta(asset, true),
            meta(this.collection, true),
            meta(this.provider.publicKey, true, true),
            meta(owner.publicKey, false, true),
            meta(SystemProgram.programId, false),
            meta(MPL_CORE_ID, false),
          ],
          data,
        }),
      ],
      [owner],
    );
  }

  static readonly THAW = Buffer.from([UPDATE_PLUGIN_V1, FREEZE_DELEGATE, 0]);
  static readonly REMOVE_FREEZE = Buffer.from([
    REMOVE_PLUGIN_V1,
    FREEZE_DELEGATE,
  ]);
  static readonly REVOKE_FREEZE = Buffer.from([
    REVOKE_PLUGIN_AUTHORITY_V1,
    FREEZE_DELEGATE,
  ]);

  assetOwner(asset: PublicKey): PublicKey {
    const account = this.provider.getAccountInfo(asset);
    if (!account) {
      throw new Error(`asset ${asset.toBase58()} does not exist`);
    }
    return new PublicKey(account.data.subarray(1, 33));
  }

  readConfig() {
    return this.program.coder.accounts.decode(
      "stakeConfig",
      this.provider.getAccountInfo(this.config)!.data,
    );
  }

  readStakeRecord(asset: PublicKey) {
    return this.program.coder.accounts.decode(
      "stakeRecord",
      this.provider.getAccountInfo(this.stakeRecord(asset))!.data,
    );
  }

  exists(account: PublicKey): boolean {
    return this.provider.getAccountInfo(account) !== null;
  }

  now(): bigint {
    return this.provider.svm.getClock().unixTimestamp;
  }

  warp(seconds: bigint): void {
    const clock = this.provider.svm.getClock();
    clock.unixTimestamp = clock.unixTimestamp + seconds;
    this.provider.svm.setClock(clock);
  }

  send(instructions: TransactionInstruction[], signers: Signer[] = []): void {
    this.provider.sendSync(new Transaction().add(...instructions), signers);
  }
}
