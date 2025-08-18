pub mod constant;
pub mod context;
pub mod error;
pub mod external;
pub mod shared;
pub mod state;

use anchor_lang::prelude::*;
use anchor_spl::token::ID as TOKEN_PROGRAM_ID;
use solana_program::{
  keccak::hash,
  program_pack::Pack,
  system_program::ID as SYSTEM_PROGRAM_ID
};
use std::convert::TryInto;

use mpl_token_metadata::accounts::Metadata;
use crate::context::*;
use crate::error::ErrorCode;
use crate::external::anchor_spl_system::transfer_lamport;
use crate::external::anchor_spl_token::transfer_token;
use crate::external::spl_token::TokenAccount;
use crate::state::{
  ObjType,
  RedemptionMultiParams,
  Schedule,
  Vault,
};
use crate::{
  constant::{SIGNER_SEED_1},
  state::{RedemptionMultiParamsV2, RedemptionNFTParams, RedemptionParams, RedemptionParamsV2},
};

#[cfg(feature = "mainnet")]
declare_id!("VLT2aFKdnYyseZgjWcW5TNu9gLMCUiRuQNZN5FhK45Q");

#[cfg(feature = "devnet")]
declare_id!("VT2uRTAsYJRavhAVcvSjk9TzyNeP1ccA6KUUD5JxeHj");

#[cfg(all(not(feature = "mainnet"), not(feature = "devnet")))]
declare_id!("7fCiqPGJdD254RS3iUYFHL1ACtqFX78YXHwYhkbLWpXY");

#[program]
mod coin98_vault {
  use super::*;

  pub fn create_vault(
  ctx: Context<CreateVaultContext>,
    _vault_path: Vec<u8>,
  ) -> Result<()> {

    let owner = &ctx.accounts.owner;

    let vault = &mut ctx.accounts.vault;

    vault.obj_type = ObjType::Vault;
    let (_, signer_nonce) = Pubkey::find_program_address(
      &[
        SIGNER_SEED_1,
        &vault.key().to_bytes(),
      ],
      ctx.program_id,
    );
    vault.signer_nonce = signer_nonce;
    vault.owner = *owner.key;
    vault.new_owner = anchor_lang::system_program::ID; // Set to empty

    Ok(())
  }

  #[access_control(is_owner(&ctx.accounts.owner.key, &ctx.accounts.vault))]
  pub fn set_vault(
    ctx: Context<SetVaultContext>,
    admins: Vec<Pubkey>,
  ) -> Result<()> {

    let vault = &mut ctx.accounts.vault;

    vault.admins = admins;

    Ok(())
  }

  #[access_control(is_admin(&ctx.accounts.admin.key, &ctx.accounts.vault))]
  pub fn create_schedule(
    ctx: Context<CreateScheduleContext>,
    user_count: u16,
    event_id: u64,
    timestamp: i64,
    merkle_root: [u8; 32],
    schedule_type: u8,
    receiving_token_mint: Pubkey,
    receiving_token_account: Pubkey,
    sending_token_mint: Pubkey,
    sending_token_account: Pubkey,
  ) -> Result<()> {

    let vault = &ctx.accounts.vault;

    let schedule = &mut ctx.accounts.schedule;

    if schedule_type == 0 {
      schedule.obj_type = ObjType::Distribution;
    } else if schedule_type == 1 {
      schedule.obj_type = ObjType::DistributionMulti;
    } else if schedule_type == 2 {
      schedule.obj_type = ObjType::NFTDistribution;
    } else if schedule_type == 3 {
      schedule.obj_type = ObjType::NFTCollectionDistribution;
    } else {
      return Err(ErrorCode::InvalidScheduleType.into());
    }
    schedule.nonce = ctx.bumps.schedule;
    schedule.event_id = event_id;
    schedule.vault_id = vault.key();
    schedule.timestamp = timestamp;
    schedule.merkle_root = merkle_root.try_to_vec().unwrap();
    schedule.receiving_token_mint = receiving_token_mint;
    schedule.receiving_token_account = receiving_token_account;
    schedule.sending_token_mint = sending_token_mint;
    schedule.sending_token_account = sending_token_account;
    schedule.is_active = true;
    schedule.redemptions = vec![false; user_count.into()];

    Ok(())
  }

  #[access_control(is_admin(&ctx.accounts.admin.key, &ctx.accounts.vault))]
  pub fn set_schedule_status(
    ctx: Context<SetScheduleContext>,
    is_active: bool,
  ) -> Result<()> {

    let schedule = &mut ctx.accounts.schedule;

    schedule.is_active = is_active;

    Ok(())
  }

  #[access_control(is_admin(&ctx.accounts.admin.key, &ctx.accounts.vault))]
  pub fn withdraw_sol(
    ctx: Context<WithdrawSolContext>,
    amount: u64,
  ) -> Result<()> {

    let vault = &ctx.accounts.vault;
    let vault_signer = &ctx.accounts.vault_signer;
    let recipient = &ctx.accounts.recipient;

    let seeds: &[&[_]] = &[
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
      &[vault.signer_nonce],
    ];
    transfer_lamport(
      &vault_signer,
      &recipient,
      amount,
      &[&seeds]
    )
    .expect("Coin98Vault: CPI failed.");

    Ok(())
  }

  #[access_control(is_admin(&ctx.accounts.admin.key, &ctx.accounts.vault))]
  pub fn withdraw_token(
    ctx: Context<WithdrawTokenContext>,
    amount: u64,
  ) -> Result<()> {
    msg!("Coin98Vault: Instruction_WithdrawToken");

    let vault = &ctx.accounts.vault;
    let vault_signer = &ctx.accounts.vault_signer;
    let sender = &ctx.accounts.sender;
    let recipient = &ctx.accounts.recipient;

    let seeds: &[&[_]] = &[
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
      &[vault.signer_nonce],
    ];
    transfer_token(
      &vault_signer,
      &sender,
      &recipient,
      amount,
      &[&seeds]
    )
    .expect("Coin98Vault: CPI failed.");

    Ok(())
  }

  #[access_control(verify_schedule(&ctx.accounts.schedule, ObjType::Distribution))]
  pub fn redeem_token<'a>(
    ctx: Context<'_, '_, '_, 'a, RedeemTokenContext<'a>>,
    index: u16,
    timestamp: i64,
    proofs: Vec<[u8; 32]>,
    receiving_amount: u64,
    sending_amount: u64,
  ) -> Result<()> {
    msg!("Coin98Vault: Instruction_RedeemToken");

    let vault = &ctx.accounts.vault;
    let vault_signer = &ctx.accounts.vault_signer;
    let accounts = &ctx.remaining_accounts;
    let user = &ctx.accounts.user;
    let vault_token0 = &ctx.accounts.vault_token0;
    let user_token0 = &ctx.accounts.user_token0;
    let clock = Clock::get().unwrap();

    let schedule = &mut ctx.accounts.schedule;
    if schedule.timestamp > 0 {
      // older version of merkle node
      require!(clock.unix_timestamp >= schedule.timestamp, ErrorCode::ScheduleLocked);
      verify_proof(index, None, &ctx.accounts.user.key, receiving_amount, sending_amount, &proofs, &schedule)?;
    } else {
      // version 2 of merkle node
      require!(clock.unix_timestamp >= timestamp, ErrorCode::ScheduleLocked);
      verify_proof(index, Some(timestamp), &ctx.accounts.user.key, receiving_amount, sending_amount, &proofs, &schedule)?;
    }

    let user_index: usize = index.into();
    schedule.redemptions[user_index] = true;

    let seeds: &[&[_]] = &[
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
      &[vault.signer_nonce],
    ];
    let result = sending_token(schedule, accounts, user, seeds, vault_signer, vault_token0, user_token0, sending_amount, receiving_amount);

    // Verify the result of sending token
    if result.is_err() {
      return Err(ErrorCode::SendingTokenFailed.into());
    }

    Ok(())
  }

  #[access_control(verify_schedule(&ctx.accounts.schedule, ObjType::DistributionMulti))]
  pub fn redeem_token_multi<'a>(
    ctx: Context<'_, '_, '_, 'a, RedeemTokenMultiContext<'a>>,
    index: u16,
    timestamp: i64,
    proofs: Vec<[u8; 32]>,
    receiving_token_mint: Pubkey,
    receiving_amount: u64,
    sending_amount: u64,
  ) -> Result<()> {
    let clock = Clock::get().unwrap();

    let vault = &ctx.accounts.vault;
    let vault_signer = &ctx.accounts.vault_signer;
    let accounts = &ctx.remaining_accounts;
    let user = &ctx.accounts.user;

    let schedule = &mut ctx.accounts.schedule;

    if schedule.timestamp > 0 {
      require!(clock.unix_timestamp >= schedule.timestamp, ErrorCode::ScheduleLocked);
      verify_proof_multi(index, None, &ctx.accounts.user.key, receiving_token_mint, receiving_amount, sending_amount, &proofs, schedule)?;
    } else {
      require!(clock.unix_timestamp >= timestamp, ErrorCode::ScheduleLocked);
      verify_proof_multi(index, Some(timestamp), &ctx.accounts.user.key, receiving_token_mint, receiving_amount, sending_amount, &proofs, schedule)?;
    }

    let user_index: usize = index.into();
    schedule.redemptions[user_index] = true;

    if schedule.sending_token_mint != solana_program::system_program::ID && sending_amount > 0 {
      let vault_token1 = &accounts[0];
      require_keys_eq!(*vault_token1.key, schedule.sending_token_account, ErrorCode::InvalidAccount);
      let user_token1 = &accounts[1];
      transfer_token(
        &user,
        &user_token1,
        &vault_token1,
        sending_amount,
        &[]
      )
      .expect("Coin98Vault: CPI failed.");
    }

    let seeds: &[&[_]] = &[
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
      &[vault.signer_nonce],
    ];
        if receiving_token_mint == SYSTEM_PROGRAM_ID {
      transfer_lamport(
        &vault_signer,
        &user,
        receiving_amount,
        &[&seeds]
      ).expect("Coin98Vault: CPI failed.");
    } else {
      let vault_token0 = &ctx.accounts.vault_token0;
      let vault_token0_account = TokenAccount::unpack_from_slice(&vault_token0.try_borrow_data().unwrap()).unwrap();
      let user_token0 = &ctx.accounts.user_token0;
      let user_token0_account = TokenAccount::unpack_from_slice(&user_token0.try_borrow_data().unwrap()).unwrap();

      require_keys_eq!(vault_token0_account.mint, receiving_token_mint, ErrorCode::InvalidAccount);
      require_keys_eq!(user_token0_account.mint, receiving_token_mint, ErrorCode::InvalidAccount);

      transfer_token(
        &vault_signer,
        &vault_token0,
        &user_token0,
        receiving_amount,
        &[&seeds]
      ).expect("Coin98Vault: CPI failed.");
    }
    Ok(())
  }

  #[access_control(verify_schedule(&ctx.accounts.schedule, ObjType::NFTDistribution))]
  pub fn redeem_token_nft<'a>(
    ctx: Context<'_, '_, '_, 'a, RedeemTokenNFTContext<'a>>,
    index: u16,
    timestamp: i64,
    nft_mint: Pubkey,
    nft_collection: Pubkey,
    receiving_amount: u64,
    sending_amount: u64,
    proofs: Vec<[u8; 32]>,
  ) -> Result<()> {
    msg!("Coin98Vault: Instruction_RedeemTokenNFT");
    let user = &ctx.accounts.user;
    let vault = &ctx.accounts.vault;
    let vault_signer = &ctx.accounts.vault_signer;
    let vault_token0 = &ctx.accounts.vault_token0;
    let user_token0 = &ctx.accounts.user_token0;
    let user_nft_token_account = &ctx.accounts.user_nft_token_account;
    let nft_metadata_account = &ctx.accounts.nft_metadata_account;
    let schedule = &mut ctx.accounts.schedule;
    let accounts = &ctx.remaining_accounts;
    let clock = Clock::get().unwrap();

    // Verify NFT ownership
    verify_nft_ownership_and_collection(
      &user.key,
      &nft_mint,
      &nft_collection,
      user_nft_token_account,
      nft_metadata_account,
    )?;

    // Verify merkle proof
    require!(clock.unix_timestamp >= timestamp, ErrorCode::ScheduleLocked);
    verify_proof_nft_collection(
      "specific".to_string(),
      index,
      timestamp,
      &nft_mint,
      &nft_collection,
      receiving_amount,
      sending_amount,
      &proofs,
      &schedule,
    )?;

    let user_index: usize = index.into();
    require!( schedule.redemptions[user_index] == false, ErrorCode::Redeemed );

    schedule.redemptions[user_index] = true;

    let seeds: &[&[_]] = &[
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
      &[vault.signer_nonce],
    ];
    let result = sending_token(schedule, accounts, user, seeds, vault_signer, vault_token0, user_token0, sending_amount, receiving_amount);

    // Verify the result of sending token
    if result.is_err() {
      return Err(ErrorCode::SendingTokenFailed.into());
    }

    Ok(())
  }

  #[access_control(verify_schedule(&ctx.accounts.schedule, ObjType::NFTCollectionDistribution))]
  pub fn redeem_token_nft_collection<'a>(
    ctx: Context<'_, '_, '_, 'a, RedeemTokenNFTCollectionContext<'a>>,
    index: u16,
    timestamp: i64,
    nft_mint: Pubkey,
    nft_collection: Pubkey,
    receiving_amount: u64,
    sending_amount: u64,
    proofs: Vec<[u8; 32]>,
  ) -> Result<()> {
    msg!("Coin98Vault: Instruction_RedeemTokenNFTCollection");
    let user = &ctx.accounts.user;
    let vault = &ctx.accounts.vault;
    let vault_signer = &ctx.accounts.vault_signer;
    let vault_token0 = &ctx.accounts.vault_token0;
    let user_token0 = &ctx.accounts.user_token0;
    let user_nft_token_account = &ctx.accounts.user_nft_token_account;
    let nft_metadata_account = &ctx.accounts.nft_metadata_account;
    let redeem_index = &mut ctx.accounts.redeem_index;
    let schedule = &mut ctx.accounts.schedule;
    let accounts = &ctx.remaining_accounts;
    let clock = Clock::get().unwrap();

    // Verify NFT ownership and collection
    verify_nft_ownership_and_collection(
      &user.key,
      &nft_mint,
      &nft_collection,
      user_nft_token_account,
      nft_metadata_account,
    )?;

    // Verify merkle proof
    require!(clock.unix_timestamp >= timestamp, ErrorCode::ScheduleLocked);
    verify_proof_nft_collection(
      "collection".to_string(),
      index,
      timestamp,
      &SYSTEM_PROGRAM_ID,
      &nft_collection,
      receiving_amount,
      sending_amount,
      &proofs,
      &schedule,
    )?;

    if redeem_index.is_redeemed {
      return Err(ErrorCode::Redeemed.into());
    } else {
      redeem_index.is_redeemed = true;
    }

    let seeds: &[&[_]] = &[
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
      &[vault.signer_nonce],
    ];
    let result = sending_token(schedule, accounts, user, seeds, vault_signer, vault_token0, user_token0, sending_amount, receiving_amount);

    // Verify the result of sending token
    if result.is_err() {
      return Err(ErrorCode::SendingTokenFailed.into());
    }

    Ok(())
  }

  pub fn init_redeem_index(
    ctx: Context<InitRedeemIndexContext>,
    _index: u16,
    _nft_mint: Pubkey,
  ) -> Result<()> {

    let redeem_index = &mut ctx.accounts.redeem_index;
    redeem_index.is_redeemed = false;

    Ok(())
  }

  #[access_control(is_owner(&ctx.accounts.owner.key, &ctx.accounts.vault))]
  pub fn transfer_ownership(
    ctx: Context<TransferOwnershipContext>,
    new_owner: Pubkey,
  ) -> Result<()> {

    let vault = &mut ctx.accounts.vault;

    vault.new_owner = new_owner;

    Ok(())
  }

  #[access_control(verify_new_owner(&ctx.accounts.new_owner.key, &ctx.accounts.vault))]
  pub fn accept_ownership(
    ctx: Context<AcceptOwnershipContext>,
  ) -> Result<()> {

    let vault = &mut ctx.accounts.vault;

    vault.owner = vault.new_owner;
    vault.new_owner = anchor_lang::system_program::ID; // Set to empty

    Ok(())
  }
}

/// Returns true if the user has root priviledge of the vault
pub fn is_owner(user: &Pubkey, vault: &Vault) -> Result<()> {

  require_keys_eq!(*user, vault.owner, ErrorCode::Unauthorized);

  Ok(())
}

/// Returns true if the user is the newly apppointed owner of the vault
pub fn verify_new_owner(user: &Pubkey, vault: &Vault) -> Result<()> {

  require_keys_eq!(*user, vault.new_owner, ErrorCode::Unauthorized);

  Ok(())
}

/// Returns true if the user is an admin of a specified vault
pub fn is_admin(user: &Pubkey, vault: &Vault) -> Result<()> {
  if *user == vault.owner {
    return Ok(());
  }

  let result = vault.admins.iter().position(|&key| key == *user);
  if result == None {
    return Err(ErrorCode::Unauthorized.into());
  }

  Ok(())
}

pub fn verify_schedule(schedule: &Schedule, expected_type: ObjType) -> Result<()> {
  require!(schedule.obj_type == expected_type, ErrorCode::WrongScheduleObjectType);
  require!(schedule.is_active, ErrorCode::ScheduleUnavailable);

  Ok(())
}

pub fn verify_proof(index: u16, timestamp: Option<i64>, user: &Pubkey, receiving_amount: u64, sending_amount: u64, proofs: &Vec<[u8; 32]>, schedule: &Schedule) -> Result<()> {
  let redemption_data = match timestamp {
    Some(timestamp) => { // if timestamp field exists on merkle node
      msg!("Vault V2");
      let redemption_params = RedemptionParamsV2 {
        index: index,
        timestamp,
        address: *user,
        receiving_amount: receiving_amount,
        sending_amount: sending_amount,
      };
      redemption_params.try_to_vec().unwrap()
    },
    None => { // older version of merkle node
      msg!("Old version vault");
      let redemption_params = RedemptionParams {
        index: index,
        address: *user,
        receiving_amount: receiving_amount,
        sending_amount: sending_amount,
      };
      redemption_params.try_to_vec().unwrap()
    }
  };

  let root: [u8; 32] = schedule.merkle_root.clone().try_into().unwrap();
  let leaf = hash(&redemption_data[..]);
  let is_valid_proof = shared::verify_proof(proofs.to_vec(), root, leaf.to_bytes());
  require!(is_valid_proof, ErrorCode::InvalidProof);

  let user_index: usize = index.into();
  require!(schedule.redemptions[user_index] == false, ErrorCode::Redeemed);

  Ok(())
}

pub fn verify_proof_multi(index: u16, timestamp: Option<i64>, user: &Pubkey, receiving_token_mint: Pubkey, receiving_amount: u64, sending_amount: u64, proofs: &Vec<[u8; 32]>, schedule: &Schedule) -> Result<()> {
  let redemption_data = match timestamp {
    Some(timestamp) => { // newer version if timestamp field exists on merkle node
      let redemption_params = RedemptionMultiParamsV2 {
        index: index,
        timestamp,
        address: *user,
        receiving_token_mint: receiving_token_mint,
        receiving_amount: receiving_amount,
        sending_amount: sending_amount,
      };
      redemption_params.try_to_vec().unwrap()
    },
    None => { // older version of merkle node
      let redemption_params = RedemptionMultiParams {
        index: index,
        address: *user,
        receiving_token_mint: receiving_token_mint,
        receiving_amount: receiving_amount,
        sending_amount: sending_amount,
      };
      redemption_params.try_to_vec().unwrap()
    }
  };

  let root: [u8; 32] = schedule.merkle_root.clone().try_into().unwrap();
  let leaf = hash(&redemption_data[..]);
  let is_valid_proof = shared::verify_proof(proofs.to_vec(), root, leaf.to_bytes());
  require!(is_valid_proof, ErrorCode::InvalidProof);

  let user_index: usize = index.into();
  require!(schedule.redemptions[user_index] == false, ErrorCode::Redeemed);

  Ok(())
}

pub fn sending_token<'a>(
  schedule: &Schedule,
  accounts: &[AccountInfo<'a>],
  user: &AccountInfo<'a>,
  seeds: &[&[u8]],
  vault_signer: &AccountInfo<'a>,
  vault_token0: &AccountInfo<'a>,
  user_token0: &AccountInfo<'a>,
  sending_amount: u64,
  receiving_amount: u64,
) -> Result<()> {
  if schedule.sending_token_mint != SYSTEM_PROGRAM_ID && sending_amount > 0 {
    let vault_token1 = &accounts[0];
    require_keys_eq!(*vault_token1.key, schedule.sending_token_account, ErrorCode::InvalidAccount);
    let user_token1 = &accounts[1];
    transfer_token(
      &user,
      &user_token1,
      &vault_token1,
      sending_amount,
      &[]
    )
    .expect("Coin98Vault: CPI failed.");
  }

  if schedule.receiving_token_mint == SYSTEM_PROGRAM_ID {
    transfer_lamport(
      &vault_signer,
      &user,
      receiving_amount,
      &[&seeds]
    ).expect("Coin98Vault: CPI failed.");
  } else {
    transfer_token(
      &vault_signer,
      &vault_token0,
      &user_token0,
      receiving_amount,
      &[&seeds]
    ).expect("Coin98Vault: CPI failed.");
  }

  Ok(())
}

/// Verify merkle proof for NFT collection-based redemption
pub fn verify_proof_nft_collection(
  redeem_type: String,
  index: u16,
  timestamp: i64,
  nft_mint: &Pubkey,
  collection_mint: &Pubkey,
  receiving_amount: u64,
  sending_amount: u64,
  proofs: &Vec<[u8; 32]>,
  schedule: &Schedule,
) -> Result<()> {
    // Always use Vault NFT Collection flow
    msg!("Vault NFT Collection");
    let redemption_params = RedemptionNFTParams {
      redeem_type,
      index,
      timestamp,
      nft_mint: *nft_mint,
      collection_mint: *collection_mint,
      receiving_amount,
      sending_amount,
    };
    let redemption_data = redemption_params.try_to_vec().unwrap();

    let root: [u8; 32] = schedule.merkle_root.clone().try_into().unwrap();
    let leaf = hash(&redemption_data[..]);
    let is_valid_proof = shared::verify_proof(proofs.to_vec(), root, leaf.to_bytes());
    require!(is_valid_proof, ErrorCode::InvalidProof);

    Ok(())
}

pub fn verify_nft_ownership_and_collection(
  user: &Pubkey,
  nft_mint: &Pubkey,
  expected_collection: &Pubkey,
  user_nft_token_account: &AccountInfo,
  nft_metadata_account: &AccountInfo,
) -> Result<()> {
  // Step 1: Verify NFT ownership
  // Unpack the token account to verify ownership details
  let token_account_data = user_nft_token_account.try_borrow_data().map_err(|_| { ErrorCode::InvalidAccount })?;

  let token_account = TokenAccount::unpack_from_slice(&token_account_data).map_err(|_e| { ErrorCode::InvalidAccount })?;

  // Verify the token account belongs to the user
  if token_account.owner != *user {
    return Err(ErrorCode::Unauthorized.into());
  }

  // Verify the token account is for the correct NFT mint
  if token_account.mint != *nft_mint {
    return Err(ErrorCode::InvalidMintAccount.into());
  }

  // For NFTs, verify that this follows NFT standards (amount = 1)
  if token_account.amount != 1 {
    return Err(ErrorCode::InvalidTokenAmount.into());
  }

  // Simplified unpacking and verification of metadata account
  let metadata_data = nft_metadata_account.try_borrow_data().map_err(|_| ErrorCode::InvalidMetadata)?;
  let metadata = Metadata::safe_deserialize(&metadata_data).map_err(|_| ErrorCode::InvalidMetadata)?;

  // Verify the NFT belongs to the expected collection
  if let Some(collection) = metadata.collection {
    if collection.key != *expected_collection {
      return Err(ErrorCode::InvalidCollection.into());
    }
  } else {
    return Err(ErrorCode::InvalidCollection.into());
  }

  Ok(())
}

pub fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
  let ata_program: Pubkey = Pubkey::new_from_array([
    140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153,
    218, 255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
  ]);
  Pubkey::find_program_address(
    &[
      &wallet.to_bytes(),
      &TOKEN_PROGRAM_ID.to_bytes(),
      &mint.to_bytes(),
    ],
    &ata_program,
  )
  .0
}

pub fn find_metadata_account(mint: &Pubkey) -> (Pubkey, u8) {
  Pubkey::find_program_address(
    &[
      Metadata::PREFIX,
      mpl_token_metadata::ID.as_ref(),
      mint.as_ref(),
    ],
    &mpl_token_metadata::ID,
  )
}
