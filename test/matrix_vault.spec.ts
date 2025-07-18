import { SolanaConfigService } from "@coin98/solana-support-library/config";
import {
  mintNft,
  updateMetadata,
  verifyCollection,
} from "./fixtures";
import {
  Connection,
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
} from "@solana/web3.js";
import {
  Account,
  createMint,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import {
  Collection,
} from "@metaplex-foundation/mpl-token-metadata";
import BN from "bn.js";
import {
  currentTime,
  expectSuccess,
  getCollectionOwner,
  getMetadataAddress,
} from "./util";
import { expect } from "chai";
import { VaultService } from "../services";
import {
  MerkleTree,
  TokenProgramService,
} from "@coin98/solana-support-library";
import { MerkleDistributionNftService } from "../services/merkle_distributor.service";

const connection = new Connection("http://127.0.0.1:8899", "confirmed");
const PROGRAM_ID = new PublicKey("7fCiqPGJdD254RS3iUYFHL1ACtqFX78YXHwYhkbLWpXY");
let payer: Keypair;
let vaultName: string;

let root: Keypair;
let user2: Keypair;
let user3: Keypair;
let mint: PublicKey;
let rootATA: Account;
let user2ATA: Account;
let user3ATA: Account;
const collectionMint: Keypair = Keypair.generate();
let nftMints: PublicKey[] = [];

let sendingTokenMint: Keypair;
let receivingTokenMint: Keypair;
let vaultAddress: PublicKey;
let scheduleAddress: PublicKey;
let tree: MerkleTree;
let snapshot = currentTime();

let eventId: BN;

describe("Vault for specific NFT", () => {
  before(async () => {
    {
      vaultName = (Math.random() + 1).toString(36).substring(7);
      console.log("Creating root account...");
      // Get root address
      root = await SolanaConfigService.getDefaultAccount();
      console.log("Root account: ", root.publicKey.toBase58());

      // Create user 2
      user2 = Keypair.generate();
      console.log("User 2: ", user2.publicKey.toBase58());

      // Airdrop to user 2
      await connection.requestAirdrop(user2.publicKey, LAMPORTS_PER_SOL);

      user3 = Keypair.generate();
      console.log("User 3: ", user3.publicKey.toBase58());

      // Airdrop to user 3
      const airdropSignature = await connection.requestAirdrop(
        user3.publicKey,
        LAMPORTS_PER_SOL
      );

      await connection.confirmTransaction(airdropSignature);

      // Create mint
      mint = await createMint(
        connection,
        root,
        root.publicKey,
        root.publicKey,
        0
      );
      console.log("Mint created: ", mint.toBase58());

      // Create root associated token account
      rootATA = await getOrCreateAssociatedTokenAccount(
        connection,
        root,
        mint,
        root.publicKey
      );
      console.log("Root ATA created: ", rootATA.address.toBase58());

      // Create user 2 associated token account
      user2ATA = await getOrCreateAssociatedTokenAccount(
        connection,
        user2,
        mint,
        user2.publicKey
      );
      console.log("User 2 ATA created: ", user2ATA.address.toBase58());

      // Create user 3 associated token account
      user3ATA = await getOrCreateAssociatedTokenAccount(
        connection,
        user3,
        mint,
        user3.publicKey
      );
      console.log("User 3 ATA created: ", user3ATA.address.toBase58());

      // Create some token to root ATA
      sendingTokenMint = await TokenProgramService.createTokenMint(
        connection,
        root,
        Keypair.generate(),
        8,
        root.publicKey,
        PROGRAM_ID
      );
      receivingTokenMint = await TokenProgramService.createTokenMint(
        connection,
        root,
        Keypair.generate(),
        8,
        root.publicKey,
        PROGRAM_ID
      );
    }
  });

  it("Create nft collection", async () => {
    let collectionMetadata = getMetadataAddress(collectionMint.publicKey);

    const mintTx = await mintNft(
      connection,
      root,
      collectionMint,
      root,
      null,
      [
        {
          address: root.publicKey,
          share: 100,
          verified: false,
        },
      ],
      "Coin98 NFT",
      5,
      "C98NFT",
      "https://arweave.net/y5e5DJsiwH0s_ayfMwYk-SnrZtVZzHLQDSTZ5dNRUHA",
      null,
      {
        __kind: "V1",
        size: 5,
      },
      true,
      new BN(0)
    );

    expectSuccess(mintTx);

    const updateTx = await updateMetadata(
      connection,
      root,
      collectionMetadata,
      null,
      null,
      root.publicKey,
      false
    );
    expectSuccess(updateTx);

    const collectionOwnerNow = await getCollectionOwner(
      connection,
      collectionMint.publicKey.toString()
    );
    console.log("Collection Owner: ", collectionOwnerNow);

    const collection: Collection = {
      key: collectionMint.publicKey,
      verified: false,
    };

    // Mint multiple NFTs to the collection
    for (let i = 0; i < 5; i++) {
      const nftMint = Keypair.generate();
      const nftMetadata = await mintNft(
        connection,
        root,
        nftMint,
        root,
        collection,
        [
          {
            address: root.publicKey,
            share: 100,
            verified: false,
          },
        ],
        `Coin98 NFT #${i + 1}`,
        0,
        "C98NFT",
        "https://arweave.net/y5e5DJsiwH0s_ayfMwYk-SnrZtVZzHLQDSTZ5dNRUHA",
        null,
        null,
        true,
        new BN(0)
      );

      expectSuccess(nftMetadata);
      // Verify collection
      const verifyTx = await verifyCollection(
        connection,
        root,
        nftMint.publicKey,
        collectionMint.publicKey
      );
      expectSuccess(verifyTx);
      nftMints.push(nftMint.publicKey);
    }
  });

  it("Transfer NFT mint to user", async () => {
    // Get Root ATA for mint
    const rootTokenAccountInfo1 =
      TokenProgramService.findAssociatedTokenAddress(
        root.publicKey,
        nftMints[0]
      );
    console.log("Root Token Account Info: ", rootTokenAccountInfo1.toBase58());

    const transferTx = await TokenProgramService.transfer(
      connection,
      root,
      rootTokenAccountInfo1,
      user2.publicKey,
      new BN(1)
    );

    const user2TokenAccountInfo = await TokenProgramService.getTokenAccountInfo(
      connection,
      TokenProgramService.findAssociatedTokenAddress(
        user2.publicKey,
        nftMints[0]
      )
    );

    expect(user2TokenAccountInfo.amount.toString()).to.equal("1");

    // Transfer NFT mint to user 3

    const rootTokenAccountInfo2 =
      TokenProgramService.findAssociatedTokenAddress(
        root.publicKey,
        nftMints[1]
      );
    console.log("Root Token Account Info: ", rootTokenAccountInfo2.toBase58());
    const user3TokenAccountInfo =
      TokenProgramService.findAssociatedTokenAddress(
        user3.publicKey,
        nftMints[1]
      );
    console.log(
      "User 3 Token Account Info: ",
      user3TokenAccountInfo.toBase58()
    );

    const transferTx2 = await TokenProgramService.transfer(
      connection,
      root,
      rootTokenAccountInfo2,
      user3.publicKey,
      new BN(1)
    );

    const user3TokenAccountInfo2 =
      await TokenProgramService.getTokenAccountInfo(
        connection,
        user3TokenAccountInfo
      );
    expect(user3TokenAccountInfo2.amount.toString()).to.equal("1");
  });

  it("Create vault", async () => {
    vaultAddress = await VaultService.createVault(
      connection,
      root,
      vaultName,
      PROGRAM_ID
    );

    const vaultInfo = await VaultService.getVaultAccountInfo(
      connection,
      vaultAddress
    );
    expect(vaultInfo.owner.toBase58()).to.equal(root.publicKey.toBase58());
  });

  it("Set vault data", async () => {
    await VaultService.setVault(
      connection,
      root,
      vaultAddress,
      [root.publicKey],
      PROGRAM_ID
    );

    const vaultInfo = await VaultService.getVaultAccountInfo(
      connection,
      vaultAddress
    );
    expect(vaultInfo.owner.toBase58()).to.equal(root.publicKey.toBase58());
  });

  it("Create schedule specific NFT", async () => {
    tree = MerkleDistributionNftService.createTree([
      {
        redeemType: "specific",
        index: 0,
        timestamp: new BN(snapshot),
        nftMint: nftMints[0],
        collectionMint: collectionMint.publicKey,
        receivingAmount: new BN(100),
        sendingAmount: new BN(0),
      },
      {
        redeemType: "specific",
        index: 1,
        timestamp: new BN(snapshot),
        nftMint: nftMints[1],
        collectionMint: collectionMint.publicKey,
        receivingAmount: new BN(100),
        sendingAmount: new BN(0),
      },
    ]);

    const vaultInfo = await VaultService.getVaultAccountInfo(
      connection,
      vaultAddress
    );
    const vaultAuthority = vaultInfo.signer;

    const vaultSendTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        vaultAuthority,
        sendingTokenMint.publicKey
      );

    const vaultReceiveTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        vaultAuthority,
        receivingTokenMint.publicKey
      );

    await TokenProgramService.mint(
      connection,
      root,
      receivingTokenMint.publicKey,
      vaultReceiveTokenAccount,
      new BN(200)
    );

    scheduleAddress = await VaultService.createSchedule(
      connection,
      root,
      vaultAddress,
      2,
      new BN(Math.random() * 1000000),
      new BN(0),
      tree.root().hash,
      new BN(2),
      receivingTokenMint.publicKey,
      vaultReceiveTokenAccount,
      sendingTokenMint.publicKey,
      vaultSendTokenAccount,
      PROGRAM_ID
    );
    console.log("Schedule Address: ", scheduleAddress.toBase58());

    const scheduleInfo = await VaultService.getScheduleAccountInfo(
      connection,
      scheduleAddress
    );
    expect(scheduleInfo.vaultId.toBase58()).to.equal(vaultAddress.toBase58());
  });

  it("Claim NFT 0", async () => {
    const proofs = MerkleDistributionNftService.getProof(tree, 0).map(
      (p) => p.hash
    );

    const userReceiveTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        user2.publicKey,
        receivingTokenMint.publicKey
      );

    const tx = await VaultService.redeemNFT(
      connection,
      user2,
      vaultAddress,
      scheduleAddress,
      0,
      new BN(snapshot),
      nftMints[0],
      collectionMint.publicKey,
      new BN(100),
      new BN(0),
      proofs,
      userReceiveTokenAccount,
      userReceiveTokenAccount,
      PROGRAM_ID
    );
  });

  it("Reclaim NFT 0", async () => {
    const proofs = MerkleDistributionNftService.getProof(tree, 0).map(
      (p) => p.hash
    );

    const userReceiveTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        user2.publicKey,
        receivingTokenMint.publicKey
      );

    const tx = await VaultService.redeemNFT(
      connection,
      user2,
      vaultAddress,
      scheduleAddress,
      0,
      new BN(snapshot),
      nftMints[0],
      collectionMint.publicKey,
      new BN(100),
      new BN(0),
      proofs,
      userReceiveTokenAccount,
      userReceiveTokenAccount,
      PROGRAM_ID
    );
  });

  it("Claim NFT 1 (not have proof)", async () => {
    const proofs = MerkleDistributionNftService.getProof(tree, 1).map(
      (p) => p.hash
    );

    const userReceiveTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        user2.publicKey,
        receivingTokenMint.publicKey
      );

    const tx = await VaultService.redeemNFT(
      connection,
      user2,
      vaultAddress,
      scheduleAddress,
      1,
      new BN(snapshot),
      nftMints[1],
      collectionMint.publicKey,
      new BN(100),
      new BN(0),
      proofs,
      userReceiveTokenAccount,
      userReceiveTokenAccount,
      PROGRAM_ID
    );
  });

  it("Create schedule for NFT collection", async () => {
    tree = MerkleDistributionNftService.createTree([
      {
        redeemType: "collection",
        index: 0,
        timestamp: new BN(snapshot),
        nftMint: SystemProgram.programId,
        collectionMint: collectionMint.publicKey,
        receivingAmount: new BN(100),
        sendingAmount: new BN(0),
      },
      {
        redeemType: "collection",
        index: 1,
        timestamp: new BN(snapshot),
        nftMint: SystemProgram.programId,
        collectionMint: collectionMint.publicKey,
        receivingAmount: new BN(100),
        sendingAmount: new BN(0),
      },
    ]);

    const vaultInfo = await VaultService.getVaultAccountInfo(
      connection,
      vaultAddress
    );
    const vaultAuthority = vaultInfo.signer;

    const vaultSendTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        vaultAuthority,
        sendingTokenMint.publicKey
      );

    const vaultReceiveTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        vaultAuthority,
        receivingTokenMint.publicKey
      );

    await TokenProgramService.mint(
      connection,
      root,
      receivingTokenMint.publicKey,
      vaultReceiveTokenAccount,
      new BN(200)
    );

    eventId = new BN(Math.random() * 1000000)
    scheduleAddress = await VaultService.createSchedule(
      connection,
      root,
      vaultAddress,
      2,
      eventId,
      new BN(0),
      tree.root().hash,
      new BN(3),
      receivingTokenMint.publicKey,
      vaultReceiveTokenAccount,
      sendingTokenMint.publicKey,
      vaultSendTokenAccount,
      PROGRAM_ID
    );
    console.log("Schedule Address: ", scheduleAddress.toBase58());

    const scheduleInfo = await VaultService.getScheduleAccountInfo(
      connection,
      scheduleAddress
    );
    expect(scheduleInfo.vaultId.toBase58()).to.equal(vaultAddress.toBase58());
  });

  it("Claim NFT collection ID 0", async () => {
    const proofs = MerkleDistributionNftService.getProof(tree, 0).map(
      (p) => p.hash
    );

    const userReceiveTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        user2.publicKey,
        receivingTokenMint.publicKey
      );

    const [isRedeemedAddress, _] = VaultService.findRedeemIndexAddress(
      eventId,
      0,
      nftMints[0],
      PROGRAM_ID
    );

    await VaultService.initRedeemIndex(
      connection,
      user2,
      vaultAddress,
      scheduleAddress,
      isRedeemedAddress,
      0,
      nftMints[0],
      PROGRAM_ID
    );

    const tx = await VaultService.redeemNFTCollection(
      connection,
      user2,
      vaultAddress,
      scheduleAddress,
      isRedeemedAddress,
      0,
      new BN(snapshot),
      nftMints[0],
      collectionMint.publicKey,
      new BN(100),
      new BN(0),
      proofs,
      userReceiveTokenAccount,
      userReceiveTokenAccount,
      PROGRAM_ID
    );
  })

  it("Reclaim NFT collection ID 0", async () => {
    const proofs = MerkleDistributionNftService.getProof(tree, 0).map(
      (p) => p.hash
    );

    const userReceiveTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        user2.publicKey,
        receivingTokenMint.publicKey
      );

    const [isRedeemedAddress, _] = VaultService.findRedeemIndexAddress(
      eventId,
      0,
      nftMints[0],
      PROGRAM_ID
    );

    const tx = await VaultService.redeemNFTCollection(
      connection,
      user2,
      vaultAddress,
      scheduleAddress,
      isRedeemedAddress,
      0,
      new BN(snapshot),
      nftMints[0],
      collectionMint.publicKey,
      new BN(100),
      new BN(0),
      proofs,
      userReceiveTokenAccount,
      userReceiveTokenAccount,
      PROGRAM_ID
    );
  });

  it("Claim NFT collection ID 1", async () => {
    const proofs = MerkleDistributionNftService.getProof(tree, 1).map(
      (p) => p.hash
    );

    const userReceiveTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        user2.publicKey,
        receivingTokenMint.publicKey
      );

    const [isRedeemedAddress, _] = VaultService.findRedeemIndexAddress(
      eventId,
      1,
      nftMints[1],
      PROGRAM_ID
    );

    await VaultService.initRedeemIndex(
      connection,
      user3,
      vaultAddress,
      scheduleAddress,
      isRedeemedAddress,
      1,
      nftMints[1],
      PROGRAM_ID
    );

    const tx = await VaultService.redeemNFTCollection(
      connection,
      user3,
      vaultAddress,
      scheduleAddress,
      isRedeemedAddress,
      1,
      new BN(snapshot),
      nftMints[1],
      collectionMint.publicKey,
      new BN(100),
      new BN(0),
      proofs,
      userReceiveTokenAccount,
      userReceiveTokenAccount,
      PROGRAM_ID
    );
  })

  it("Create schedule for native token", async () => {
    tree = MerkleDistributionNftService.createTree([
      {
        redeemType: "collection",
        index: 0,
        timestamp: new BN(snapshot),
        nftMint: SystemProgram.programId,
        collectionMint: collectionMint.publicKey,
        receivingAmount: new BN(100),
        sendingAmount: new BN(0),
      },
      {
        redeemType: "collection",
        index: 1,
        timestamp: new BN(snapshot),
        nftMint: SystemProgram.programId,
        collectionMint: collectionMint.publicKey,
        receivingAmount: new BN(100),
        sendingAmount: new BN(0),
      },
    ]);

    const vaultInfo = await VaultService.getVaultAccountInfo(
      connection,
      vaultAddress
    );

    const vaultAuthority = vaultInfo.signer;

    const vaultSendTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        vaultAuthority,
        sendingTokenMint.publicKey
      );

    const vaultReceiveTokenAccount =
      await TokenProgramService.createAssociatedTokenAccount(
        connection,
        root,
        vaultAuthority,
        receivingTokenMint.publicKey
      );

    // Sending lamport to vault receive token account
    await connection.requestAirdrop(
      vaultAuthority,
      LAMPORTS_PER_SOL
    );

    eventId = new BN(Math.random() * 1000000);

    scheduleAddress = await VaultService.createSchedule(
      connection,
      root,
      vaultAddress,
      2,
      eventId,
      new BN(0),
      tree.root().hash,
      new BN(3),
      SystemProgram.programId,
      vaultReceiveTokenAccount,
      sendingTokenMint.publicKey,
      vaultSendTokenAccount,
      PROGRAM_ID
    );
    console.log("Schedule Address: ", scheduleAddress.toBase58());

    const scheduleInfo = await VaultService.getScheduleAccountInfo(
      connection,
      scheduleAddress
    );
    expect(scheduleInfo.vaultId.toBase58()).to.equal(vaultAddress.toBase58());
  })

  it("Claim native token ID 0", async () => {
    const proofs = MerkleDistributionNftService.getProof(tree, 0).map(
      (p) => p.hash
    );

    const [isRedeemedAddress, _] = VaultService.findRedeemIndexAddress(
      eventId,
      0,
      nftMints[0],
      PROGRAM_ID
    );

    await VaultService.initRedeemIndex(
      connection,
      user2,
      vaultAddress,
      scheduleAddress,
      isRedeemedAddress,
      0,
      nftMints[0],
      PROGRAM_ID
    );

    const tx = await VaultService.redeemNFTCollection(
      connection,
      user2,
      vaultAddress,
      scheduleAddress,
      isRedeemedAddress,
      0,
      new BN(snapshot),
      nftMints[0],
      collectionMint.publicKey,
      new BN(100),
      new BN(0),
      proofs,
      user2.publicKey,
      user2.publicKey,
      PROGRAM_ID
    );
  });
});
