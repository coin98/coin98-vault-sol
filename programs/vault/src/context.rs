use anchor_lang::prelude::*;

use crate::constant::{
  REDEEM_INDEX_SEED_1,
  SCHEDULE_SEED_1,
  SIGNER_SEED_1,
  VAULT_SEED_1,
};
use crate::error::ErrorCode;
use crate::external::spl_token::is_token_program;
use crate::state::{
  ObjType,
  RedemptionIndex,
  Schedule,
  Vault
};
use crate::{
  find_metadata_account,
  get_associated_token_address,
  shared
};

#[derive(Accounts)]
#[instruction(vault_path: Vec<u8>)]
pub struct CreateVaultContext<'info> {

  /// CHECK: owner of newly vault
  #[account(signer, mut)]
  pub owner: AccountInfo<'info>,

  #[account(
    init,
    seeds = [
      &VAULT_SEED_1,
      &*vault_path,
    ],
    bump,
    payer = owner,
    space = 16 + Vault::size(),
  )]
  pub vault: Account<'info, Vault>,

  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetVaultContext<'info> {

  /// CHECK: vault owner, verified using #access_control
  #[account(signer)]
  pub owner: AccountInfo<'info>,

  #[account(mut)]
  pub vault: Account<'info, Vault>,
}

#[derive(Accounts)]
#[instruction(user_count: u16, event_id: u64)]
pub struct CreateScheduleContext<'info> {

  /// CHECK: vault admin, verified using #access_control
  #[account(signer, mut)]
  pub admin: AccountInfo<'info>,

  pub vault: Account<'info, Vault>,

  #[account(
    init,
    seeds = [
      &SCHEDULE_SEED_1,
      &shared::derive_event_id(event_id).as_ref(),
    ],
    bump,
    payer = admin,
    space = 16 + Schedule::size(user_count),
  )]
  pub schedule: Account<'info, Schedule>,

  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetScheduleContext<'info> {

  /// CHECK: vault admin, verified using #access_control
  #[account(signer, mut)]
  pub admin: AccountInfo<'info>,

  pub vault: Account<'info, Vault>,

  #[account(
    mut,
    seeds = [
      &SCHEDULE_SEED_1,
      &shared::derive_event_id(schedule.event_id).as_ref(),
    ],
    bump = schedule.nonce,
    constraint = schedule.vault_id == vault.key() @ErrorCode::InvalidAccount
  )]
  pub schedule: Account<'info, Schedule>,
}

#[derive(Accounts)]
pub struct WithdrawSolContext<'info> {

  /// CHECK: vault admin, verified using #access_control
  #[account(signer, mut)]
  pub admin: AccountInfo<'info>,

  pub vault: Account<'info, Vault>,

  /// CHECK: PDA to hold vault's assets
  #[account(
    mut,
    seeds = [
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
    ],
    bump = vault.signer_nonce
  )]
  pub vault_signer: AccountInfo<'info>,

  /// CHECK: Destination SOL account
  #[account(mut)]
  pub recipient: AccountInfo<'info>,

  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawTokenContext<'info> {

  /// CHECK: vault admin, verified using #access_control
  #[account(signer, mut)]
  pub admin: AccountInfo<'info>,

  pub vault: Account<'info, Vault>,

  /// CHECK: PDA to hold vault's assets
  #[account(
    seeds = [
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
    ],
    bump = vault.signer_nonce
  )]
  pub vault_signer: AccountInfo<'info>,

  /// CHECK: Vault's TokenAccount for distribution
  #[account(mut)]
  pub sender: AccountInfo<'info>,

  /// CHECK: Destination token account
  #[account(mut)]
  pub recipient: AccountInfo<'info>,

  /// CHECK: Solana native Token Program
  #[account(
    constraint = is_token_program(&token_program) @ErrorCode::InvalidAccount,
  )]
  pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RedeemTokenContext<'info> {

  pub vault: Account<'info, Vault>,

  #[account(
    mut,
    seeds = [
      &SCHEDULE_SEED_1,
      &shared::derive_event_id(schedule.event_id).as_ref(),
    ],
    bump = schedule.nonce,
    constraint = schedule.vault_id == vault.key() @ErrorCode::InvalidVault,
    constraint = schedule.obj_type == ObjType::Distribution @ErrorCode::WrongScheduleObjectType,
  )]
  pub schedule: Account<'info, Schedule>,

  /// CHECK: PDA to hold vault's assets
  #[account(
    seeds = [
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
    ],
    bump = vault.signer_nonce
  )]
  pub vault_signer: AccountInfo<'info>,

  /// CHECK: Program's TokenAccount for distribution
  #[account(
    mut,
    constraint = *vault_token0.key == schedule.receiving_token_account @ErrorCode::InvalidAccount
  )]
  pub vault_token0: AccountInfo<'info>,

  /// CHECK: User account eligible to redeem token. Must sign to provide proof of redemption
  #[account(signer)]
  pub user: AccountInfo<'info>,

  /// CHECK: User account to receive token
  #[account(mut)]
  pub user_token0: AccountInfo<'info>,

  /// CHECK: Solana native Token Program
  #[account(
    constraint = is_token_program(&token_program) @ErrorCode::InvalidAccount
  )]
  pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RedeemTokenMultiContext<'info> {

  pub vault: Account<'info, Vault>,

  #[account(
    mut,
    seeds = [
      &SCHEDULE_SEED_1,
      &shared::derive_event_id(schedule.event_id).as_ref(),
    ],
    bump = schedule.nonce,
    constraint = schedule.vault_id == vault.key() @ErrorCode::InvalidVault,
    constraint = schedule.obj_type == ObjType::DistributionMulti @ErrorCode::WrongScheduleObjectType,
  )]
  pub schedule: Account<'info, Schedule>,

  /// CHECK: PDA to hold vault's assets
  #[account(
    seeds = [
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
    ],
    bump = vault.signer_nonce
  )]
  pub vault_signer: AccountInfo<'info>,

  /// CHECK: Program's TokenAccount for distribution
  #[account(mut)]
  pub vault_token0: AccountInfo<'info>,

  /// CHECK: User account eligible to redeem token. Must sign to provide proof of redemption
  #[account(signer)]
  pub user: AccountInfo<'info>,

  /// CHECK: User account to receive token
  #[account(mut)]
  pub user_token0: AccountInfo<'info>,

  /// CHECK: Solana native Token Program
  #[account(
    constraint = is_token_program(&token_program) @ErrorCode::InvalidAccount
  )]
  pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct TransferOwnershipContext<'info> {

  /// CHECK: vault owner, verified using #access_control
  #[account(signer)]
  pub owner: AccountInfo<'info>,

  #[account(mut)]
  pub vault: Account<'info, Vault>,
}


#[derive(Accounts)]
#[instruction(index: u16, timestamp: i64, nft_mint: Pubkey)]
pub struct RedeemTokenNFTContext<'info> {

  pub vault: Account<'info, Vault>,

  #[account(
    mut,
    seeds = [
      &SCHEDULE_SEED_1,
      &shared::derive_event_id(schedule.event_id).as_ref(),
    ],
    bump = schedule.nonce,
    constraint = schedule.vault_id == vault.key() @ErrorCode::InvalidVault,
    constraint = schedule.obj_type == ObjType::NFTDistribution @ErrorCode::WrongScheduleObjectType,
  )]
  pub schedule: Account<'info, Schedule>,

  /// CHECK: PDA to hold vault's assets
  #[account(
    seeds = [
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
    ],
    bump = vault.signer_nonce
  )]
  pub vault_signer: AccountInfo<'info>,

  /// CHECK: Program's TokenAccount for distribution
  #[account(
    mut,
    constraint = *vault_token0.key == schedule.receiving_token_account @ErrorCode::InvalidAccount
  )]
  pub vault_token0: AccountInfo<'info>,

  /// CHECK: User account eligible to redeem token. Must sign to provide proof of redemption
  #[account(signer, mut)]
  pub user: AccountInfo<'info>,

  /// CHECK: User account to receive token
  #[account(mut)]
  pub user_token0: AccountInfo<'info>,

  /// CHECK: User's NFT token account to verify ownership
  #[account(
    mut,
    address = get_associated_token_address(&user.key(), &nft_mint)
  )]
  pub user_nft_token_account: AccountInfo<'info>,

  /// CHECK: NFT metadata account for verification
  #[account(
    mut,
    address = find_metadata_account(&nft_mint).0
  )]
  pub nft_metadata_account: AccountInfo<'info>,

  /// CHECK: Solana native Token Program
  #[account(
    constraint = is_token_program(&token_program) @ErrorCode::InvalidAccount
  )]
  pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(index: u16, timestamp: i64, nft_mint: Pubkey, nft_collection: Pubkey)]
pub struct RedeemTokenNFTCollectionContext<'info> {

  pub vault: Account<'info, Vault>,

  #[account(
    mut,
    seeds = [
      &SCHEDULE_SEED_1,
      &shared::derive_event_id(schedule.event_id).as_ref(),
    ],
    bump = schedule.nonce,
    constraint = schedule.vault_id == vault.key() @ErrorCode::InvalidVault,
    constraint = schedule.obj_type == ObjType::NFTCollectionDistribution @ErrorCode::WrongScheduleObjectType,
  )]
  pub schedule: Account<'info, Schedule>,

  #[account(
    mut,
    seeds = [
      &REDEEM_INDEX_SEED_1,
      &shared::derive_event_id(schedule.event_id).as_ref(),
      index.to_le_bytes().as_ref(),
      nft_mint.as_ref(),
    ],
    bump,
  )]
  pub redeem_index: Account<'info, RedemptionIndex>,

  /// CHECK: PDA to hold vault's assets
  #[account(
    seeds = [
      &SIGNER_SEED_1,
      vault.to_account_info().key.as_ref(),
    ],
    bump = vault.signer_nonce
  )]
  pub vault_signer: AccountInfo<'info>,

  /// CHECK: Program's TokenAccount for distribution
  #[account(
    mut,
    constraint = *vault_token0.key == schedule.receiving_token_account @ErrorCode::InvalidAccount
  )]
  pub vault_token0: AccountInfo<'info>,

  /// CHECK: User account eligible to redeem token. Must sign to provide proof of redemption
  #[account(signer, mut)]
  pub user: AccountInfo<'info>,

  /// CHECK: User account to receive token
  #[account(mut)]
  pub user_token0: AccountInfo<'info>,

  /// CHECK: User's NFT token account to verify ownership
  #[account(
    mut,
    address = get_associated_token_address(&user.key(), &nft_mint)
  )]
  pub user_nft_token_account: AccountInfo<'info>,

  /// CHECK: NFT metadata account for verification
  #[account(
    mut,
    address = find_metadata_account(&nft_mint).0
  )]
  pub nft_metadata_account: AccountInfo<'info>,

  /// CHECK: Solana native Token Program
  #[account(
    constraint = is_token_program(&token_program) @ErrorCode::InvalidAccount
  )]
  pub token_program: AccountInfo<'info>,
}

#[derive(Accounts)]
#[instruction(_index: u16, _nft_mint: Pubkey)]
pub struct InitRedeemIndexContext<'info> {

  /// CHECK: vault admin, verified using #access_control
  #[account(signer, mut)]
  pub user: AccountInfo<'info>,

  pub vault: Account<'info, Vault>,

  #[account(
    init,
    payer = user,
    seeds = [
      &REDEEM_INDEX_SEED_1,
      &shared::derive_event_id(schedule.event_id).as_ref(),
      _index.to_le_bytes().as_ref(),
      _nft_mint.as_ref(),
    ],
    bump,
    space = 8 + 1,
  )]
  pub redeem_index: Account<'info, RedemptionIndex>,

  #[account(
    seeds = [
      &SCHEDULE_SEED_1,
      &shared::derive_event_id(schedule.event_id).as_ref(),
    ],
    bump = schedule.nonce,
    constraint = schedule.vault_id == vault.key() @ErrorCode::InvalidVault,
  )]
  pub schedule: Account<'info, Schedule>,

  pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AcceptOwnershipContext<'info> {

  /// CHECK: new vault owner, verified using #access_control
  #[account(signer)]
  pub new_owner: AccountInfo<'info>,

  #[account(mut)]
  pub vault: Account<'info, Vault>,
}
