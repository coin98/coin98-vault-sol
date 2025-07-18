import {
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  TokenProgramService,
  TOKEN_PROGRAM_ID,
  SolanaService,
  INITIALIZE_MINT_SPAN,
  TokenProgramInstructionService,
  sendTransaction,
  TransactionLog,
  sendTransaction2,
} from "../../solana-support-library";
import { createTransferCheckedInstruction } from "@solana/spl-token";
import BN from "bn.js";
import {
  PROGRAM_ADDRESS,
  createCreateMetadataAccountV3Instruction,
  createCreateMasterEditionV3Instruction,
  Collection,
  Creator,
  Uses,
  createUpdateMetadataAccountV2Instruction,
  createVerifySizedCollectionItemInstruction,
  VerifySizedCollectionItemInstructionAccounts,
} from "@metaplex-foundation/mpl-token-metadata";
import { getMasterEditionAddress, getMetadataAddress } from "../util";

export interface NftCollectionFixture {
  collectionMint: PublicKey;
  nfts: PublicKey[];
}

export async function mintNft(
  connection: Connection,
  rootAccount: Keypair,
  tokenMint: Keypair,
  tokenMintAuthority: Keypair,
  collection: Collection | null,
  creators: Creator[],
  name: string,
  sellerFeeBasisPoints: number,
  symbol: string,
  uri: string,
  uses: Uses | null,
  collectionDetails: any,
  isMutable: boolean,
  maxSupply: BN
): Promise<string | any> {
  const metadata = getMetadataAddress(tokenMint.publicKey);
  const masterEdition = getMasterEditionAddress(tokenMint.publicKey);

  const transaction: Transaction = new Transaction();

  if (await SolanaService.isAddressAvailable(connection, tokenMint.publicKey)) {
    const lamportsToInitializeMint =
      await connection.getMinimumBalanceForRentExemption(INITIALIZE_MINT_SPAN);

    const createAccountInstruction = SystemProgram.createAccount({
      fromPubkey: rootAccount.publicKey,
      newAccountPubkey: tokenMint.publicKey,
      lamports: lamportsToInitializeMint,
      space: INITIALIZE_MINT_SPAN,
      programId: TOKEN_PROGRAM_ID,
    });
    transaction.add(createAccountInstruction);

    const initializeTokenMintInstruction =
      TokenProgramInstructionService.initializeMint(
        tokenMint.publicKey,
        0,
        tokenMintAuthority.publicKey,
        tokenMintAuthority.publicKey
      );
    transaction.add(initializeTokenMintInstruction);
  }

  const ownerTokenAccount: PublicKey =
    TokenProgramService.findAssociatedTokenAddress(
      rootAccount.publicKey,
      tokenMint.publicKey
    );

  if (await SolanaService.isAddressAvailable(connection, ownerTokenAccount)) {
    const createATAInstruction =
      TokenProgramInstructionService.createAssociatedTokenAccount(
        rootAccount.publicKey,
        rootAccount.publicKey,
        tokenMint.publicKey
      );
    transaction.add(createATAInstruction);
  }

  const mintInstruction = TokenProgramInstructionService.mint(
    tokenMintAuthority.publicKey,
    tokenMint.publicKey,
    ownerTokenAccount,
    new BN(1)
  );

  transaction.add(mintInstruction);

  const createCreateMetadataInstruction =
    createCreateMetadataAccountV3Instruction(
      {
        metadata: metadata,
        mint: tokenMint.publicKey,
        mintAuthority: tokenMintAuthority.publicKey,
        payer: rootAccount.publicKey,
        updateAuthority: tokenMintAuthority.publicKey,
      },
      {
        createMetadataAccountArgsV3: {
          data: {
            collection,
            creators,
            name,
            sellerFeeBasisPoints,
            symbol,
            uri,
            uses,
          },
          isMutable,
          collectionDetails,
        },
      }
    );

  transaction.add(createCreateMetadataInstruction);

  const createMasterEditionInstruction = createCreateMasterEditionV3Instruction(
    {
      edition: masterEdition,
      metadata,
      mint: tokenMint.publicKey,
      mintAuthority: tokenMintAuthority.publicKey,
      payer: rootAccount.publicKey,
      updateAuthority: tokenMintAuthority.publicKey,
    },
    {
      createMasterEditionArgs: {
        maxSupply,
      },
    }
  );

  transaction.add(createMasterEditionInstruction);

  const txSign: string | any = await sendTransaction(connection, transaction, [
    rootAccount,
    tokenMint,
    tokenMintAuthority,
  ]);

  console.log(
    `Mint new NFT ${tokenMint.publicKey.toString()}`,
    "---",
    txSign,
    "\n"
  );

  return txSign;
}

export const updateMetadata = async (
  connection: Connection,
  authority: Keypair,
  metadataAddress: PublicKey,
  data: any,
  primarySaleHappened: any,
  updateAuthority: PublicKey,
  isMutable: boolean
): Promise<[string, TransactionLog]> => {
  const transaction: Transaction = new Transaction();

  const updateMetadataAccountV2Instruction: TransactionInstruction =
    createUpdateMetadataAccountV2Instruction(
      {
        metadata: metadataAddress,
        updateAuthority: authority.publicKey,
      },
      {
        updateMetadataAccountArgsV2: {
          data,
          primarySaleHappened,
          updateAuthority,
          isMutable,
        },
      }
    );

  transaction.add(updateMetadataAccountV2Instruction);

  return sendTransaction2(connection, transaction, [authority]);
};

export async function verifyCollection(
  connection: Connection,
  authority: Keypair,
  mint: PublicKey,
  collectionMint: PublicKey
) {
  const transaction: Transaction = new Transaction();

  const metadataAddress = getMetadataAddress(mint);
  const collectionMintAddress = getMetadataAddress(collectionMint);
  const masterEditionAddress = getMasterEditionAddress(collectionMint);

  const verifySizedCollectionItemInstructionAccounts: VerifySizedCollectionItemInstructionAccounts =
    {
      metadata: metadataAddress,
      collectionAuthority: authority.publicKey,
      payer: authority.publicKey,
      collectionMint: collectionMint,
      collection: collectionMintAddress,
      collectionMasterEditionAccount: masterEditionAddress,
    };

  const verifyCollectionInstruction =
    createVerifySizedCollectionItemInstruction(
      verifySizedCollectionItemInstructionAccounts,
      new PublicKey(PROGRAM_ADDRESS)
    );

  transaction.add(verifyCollectionInstruction);

  // return sendTransaction2(connection, transaction, [authority]);
  const txSign: string | any = await sendTransaction(connection, transaction, [
    authority,
  ]);

  console.log(`Verify Collection Tx: `, txSign);

  return txSign;
}

export async function transferNft(
  connection: Connection,
  from: Keypair,
  mint: PublicKey,
  to: PublicKey,
  owner: PublicKey
) {
  const transaction: Transaction = new Transaction();

  const transferInstruction = createTransferCheckedInstruction(
    from.publicKey,
    mint,
    to,
    owner,
    1,
    0,
    [from],
    TOKEN_PROGRAM_ID
  );

  transaction.add(transferInstruction);

  return sendTransaction2(connection, transaction, [from]);
}
