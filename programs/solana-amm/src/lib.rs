pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("6muBSXBskSehAPm6kUkNiUHsJoGinxPSSWKgbyzrotGy");

#[program]
pub mod solana_amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, seed: u64, fee: u16, authority: Option<Pubkey>) -> Result<()> {
        ctx.accounts.init(seed, fee, authority, ctx.bumps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
        ctx.accounts.deposit(amount, max_x, max_y)
    }
    pub fn burn(ctx: Context<Burn>, amount: u64) -> Result<()> {
        ctx.accounts.burn(amount)
    }
    pub fn withdraw(ctx: Context<Withdraw>, lp_amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        ctx.accounts.withdraw(lp_amount, min_x, min_y)
    }

    pub fn withdraw_ix(ctx: Context<WithdrawIx>, lp_amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        ctx.accounts.withdraw_ix(lp_amount, min_x, min_y)
    }

    pub fn swap(ctx: Context<Swap>, amount_in: u64, min_amount_out: u64, is_x: bool) -> Result<()> {
        ctx.accounts.swap(amount_in, min_amount_out, is_x)
    }
}
