import { Connection, Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { SolanaConfigService, TestAccountService } from "@coin98/solana-support-library/config";
import { VaultService } from "../services/vault.service";
import { expect } from "chai";
import { BN } from "bn.js";
import "./util";
import { currentTime } from "./util";
import { MerkleDistributionService } from "../services/merkle_distributor.service";
import { MerkleTree, SolanaService, TokenProgramService } from "@coin98/solana-support-library";

const connection = new Connection("http://127.0.0.1:8899", "confirmed");
const PROGRAM_ID = new PublicKey("CfM6SDyyHxi8C76jT1i74p7znPEL6cLmkgBD8sgf9k1U");

let payer: Keypair;
let vaultName: string;
let vaultAddress: PublicKey;
let sendingTokenMint: Keypair;
let receivingTokenMint: Keypair;
let tree: MerkleTree;
let scheduleAddress: PublicKey;
let user: Keypair;
let snapshot = currentTime();

describe("Vault", () => {
  before(async () => {
    payer = await SolanaConfigService.getDefaultAccount();
    user = await TestAccountService.getAccount(1);
    await connection.requestAirdrop(user.publicKey, 1000000000);
    vaultName = (Math.random() + 1).toString(36).substring(7);
    sendingTokenMint = await TokenProgramService.createTokenMint(
      connection,
      payer,
      Keypair.generate(),
      8,
      payer.publicKey,
      PROGRAM_ID
    );
    receivingTokenMint = await TokenProgramService.createTokenMint(
      connection,
      payer,
      Keypair.generate(),
      8,
      payer.publicKey,
      PROGRAM_ID
    );
  });

  it("Create vault", async () => {
    vaultAddress = await VaultService.createVault(connection, payer, vaultName, PROGRAM_ID);

    const vaultInfo = await VaultService.getVaultAccountInfo(connection, vaultAddress);
    expect(vaultInfo.owner.toBase58()).to.equal(payer.publicKey.toBase58());
  });

  it("Set vault data", async () => {
    await VaultService.setVault(connection, payer, vaultAddress, [payer.publicKey], PROGRAM_ID);
  });

  it("Create schedule", async () => {
    tree = MerkleDistributionService.createTree([
      {
        index: 0,
        timestamp: new BN(snapshot),
        address: user.publicKey,
        sendingAmount: new BN(0),
        receivingAmount: new BN(100)
      },
      {
        index: 1,
        timestamp: new BN(snapshot),
        address: user.publicKey,
        sendingAmount: new BN(0),
        receivingAmount: new BN(100)
      }
    ]);

    const vaultInfo = await VaultService.getVaultAccountInfo(connection, vaultAddress);
    const vaultAuthority = vaultInfo.signer

    const vaultSendTokenAccount = await TokenProgramService.createAssociatedTokenAccount(
      connection,
      payer,
      vaultAuthority,
      sendingTokenMint.publicKey
    );

    const vaultReceiveTokenAccount = await TokenProgramService.createAssociatedTokenAccount(
      connection,
      payer,
      vaultAuthority,
      receivingTokenMint.publicKey
    );

    await TokenProgramService.mint(connection, payer, receivingTokenMint.publicKey, vaultReceiveTokenAccount, new BN(100));

    scheduleAddress = await VaultService.createSchedule(
      connection,
      payer,
      vaultAddress,
      3,
      new BN(Math.random() * 1000000),
      new BN(snapshot),
      tree.root().hash,
      false,
      receivingTokenMint.publicKey,
      vaultReceiveTokenAccount,
      sendingTokenMint.publicKey,
      vaultSendTokenAccount,
      PROGRAM_ID
    );
  });

  it("Get schedule", async () => {
    const schedule = await VaultService.getScheduleAccountInfo(connection, scheduleAddress);
    expect(schedule.vaultId.toBase58()).to.equal(vaultAddress.toBase58());
  });

  it("Redeem token", async () => {
    const proofs = MerkleDistributionService.getProof(tree, 0).map(item => item.hash);
    const userReceiveTokenAccount = await TokenProgramService.createAssociatedTokenAccount(
      connection,
      payer,
      user.publicKey,
      receivingTokenMint.publicKey
    );

    await VaultService.redeem(
      connection,
      user,
      vaultAddress,
      scheduleAddress,
      0,
      new BN(snapshot),
      proofs,
      new BN(100),
      new BN(0),
      userReceiveTokenAccount,
      userReceiveTokenAccount,
      PROGRAM_ID
    );
  });
});
