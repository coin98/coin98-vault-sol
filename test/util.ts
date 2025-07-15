import { Connection, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import {
  TransactionLog,
} from "@coin98/solana-support-library";
import { expect } from "chai";
import { PROGRAM_ADDRESS } from "@metaplex-foundation/mpl-token-metadata";

export function currentTime(): number {
  return Math.floor(Date.now() / 1000);
}

export const expectSuccess = (tx: [string, TransactionLog]) => {
  if (tx[0] == null) {
    console.log("📌 Transaction Logs: ", tx[1]);
  }
  expect(tx[0] != null).to.be.true;
};

export async function getCollectionOwner(
  connection: Connection,
  collectionAddress: string
) {
  const largestAccounts = await connection.getTokenLargestAccounts(
    new PublicKey(collectionAddress)
  );
  const largestAccountInfo = await connection.getParsedAccountInfo(
    largestAccounts.value[0].address
  );
  const owner = (largestAccountInfo.value?.data as any).parsed.info.owner;
  return owner;
}

export function getMetadataAddress(mint: PublicKey): PublicKey {
  const [metadata] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("metadata"),
      new PublicKey(PROGRAM_ADDRESS).toBuffer(),
      mint.toBuffer(),
    ],
    new PublicKey(PROGRAM_ADDRESS)
  );
  return metadata;
}

export function getMasterEditionAddress(mint: PublicKey): PublicKey {
  const [masterEdition] = PublicKey.findProgramAddressSync(
    [
      Buffer.from("metadata"),
      new PublicKey(PROGRAM_ADDRESS).toBuffer(),
      mint.toBuffer(),
      Buffer.from("edition"),
    ],
    new PublicKey(PROGRAM_ADDRESS)
  );
  return masterEdition;
}
