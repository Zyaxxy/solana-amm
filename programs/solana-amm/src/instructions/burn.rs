use anchor_lang::prelude::*;
use anchor_spl::token::{burn as burn_tokens, Burn as BurnCpi};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::error::AmmError;
use crate::state::Config;

#[derive(Accounts)]
pub struct Burn<'info> {
	#[account(mut)]
	pub user: Signer<'info>,

	#[account(
		seeds = [b"config", config.seed.to_le_bytes().as_ref()],
		bump = config.config_bump,
	)]
	pub config: Account<'info, Config>,
	#[account(
		mut,
		seeds = [b"lp", config.key().as_ref()],
		bump = config.lp_bump,
	)]
	pub mint_lp: Box<InterfaceAccount<'info, Mint>>,
	#[account(
		mut,
		associated_token::mint = mint_lp,
		associated_token::authority = user,
		associated_token::token_program = token_program,
	)]
	pub user_lp: Box<InterfaceAccount<'info, TokenAccount>>,
	pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Burn<'info> {
	pub fn burn(&self, amount: u64) -> Result<()> {
		require!(!self.config.locked, AmmError::PoolLocked);
		require_neq!(amount, 0, AmmError::InvalidAmount);
		require_gte!(self.user_lp.amount, amount, AmmError::InsufficientFunds);

		let cpi_program = self.token_program.key();
		let cpi_accounts = BurnCpi {
			mint: self.mint_lp.to_account_info(),
			from: self.user_lp.to_account_info(),
			authority: self.user.to_account_info(),
		};
		let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
		burn_tokens(cpi_ctx, amount)
	}
}
