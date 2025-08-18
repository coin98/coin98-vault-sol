use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {

  #[msg("Coin98Vault: Invalid account")]
  InvalidAccount,

  #[msg("Coin98Vault: Invalid input")]
  InvalidInput,

  #[msg("Coin98Vault: Redeemed.")]
  Redeemed,

  #[msg("Coin98Vault: Schedule locked.")]
  ScheduleLocked,

  #[msg("Coin98Vault: Schedule unavailable.")]
  ScheduleUnavailable,

  #[msg("Coin98Vault: Unauthorized.")]
  Unauthorized,

  #[msg("Coin98Vault: Invalid schedule type.")]
  InvalidScheduleType,

  #[msg("Coin98Vault: Invalid mint account")]
  InvalidMintAccount,

  #[msg("Coin98Vault: Invalid token amount")]
  InvalidTokenAmount,

  #[msg("Coin98Vault: Invalid metadata")]
  InvalidMetadata,

  #[msg("Coin98Vault: NFT collection mismatch.")]
  InvalidCollection,

  #[msg("Coin98Vault: Invalid proof")]
  InvalidProof,

  #[msg("Coin98Vault: Wrong schedule object type")]
  WrongScheduleObjectType,

  #[msg("Coin98Vault: Wrong vault Id")]
  InvalidVault,

  #[msg("Coin98Vault: Sending token failed")]
  SendingTokenFailed,
}

