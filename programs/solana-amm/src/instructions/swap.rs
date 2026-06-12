use anchor_lang::prelude::*;
use anchor_spl::token::{Transfer, transfer};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use anchor_spl::associated_token::AssociatedToken;

use constant_product_curve::{ConstantProduct, LiquidityPair};
use crate::state::Config;
use crate::error::AmmError;
#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    pub mint_x: Box<InterfaceAccount<'info, Mint>>,
    pub mint_y: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"config", config.seed.to_le_bytes().as_ref()],
        bump = config.config_bump,
    )]
    pub config: Account<'info, Config>,
    #[account(mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_x: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
        associated_token::token_program = token_program,
    )]
    pub vault_y: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut,
        seeds = [b"lp", config.key().as_ref()],
        bump = config.lp_bump,
    )]
    pub mint_lp: Box<InterfaceAccount<'info, Mint>>,
    #[account(mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_x: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_y: Box<InterfaceAccount<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_lp: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl <'info> Swap<'info> {
    pub fn swap(&mut self, amount_in: u64, min_amount_out: u64, is_x: bool) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require_neq!(amount_in, 0, AmmError::InvalidAmount);
        
        let mut curve = ConstantProduct::init(
           self.vault_x.amount,
           self.vault_y.amount,
           self.mint_lp.supply,
           self.config.fee,
           Some(6),
        ).unwrap();

        let p = match is_x {
            true => LiquidityPair::X,
            false => LiquidityPair::Y
        };

        let swap_result = curve.swap(p, amount_in, min_amount_out).map_err(|_| AmmError::SlippageExceeded)?;       
        self.deposit_tokens(is_x, swap_result.deposit)?;
        self.withdraw_tokens(is_x, swap_result.withdraw)
            
    }
    pub fn withdraw_tokens(&self, is_x: bool, amount: u64)-> Result<()> {
        let (from, to)= match is_x {

            true => ( 
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            ),
            false => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            ),
            
        };
        let cpi_program = self.token_program.key();
        let cpi_accounts = Transfer{
            from,
            to,
            authority: self.config.to_account_info(),
        };
        let signer_seeds: &[&[&[u8]]] =&[&[
            b"config",
            &self.config.seed.to_le_bytes(),
            &[self.config.config_bump],
        ]];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer(cpi_ctx, amount)
    }

    pub fn deposit_tokens(&mut self, is_x: bool, amount: u64) -> Result<()> {
       let (from, to) = match is_x {
            true => (
                self.user_x.to_account_info(),
                self.vault_x.to_account_info(),
            ),
            false => (
                self.user_y.to_account_info(),
                self.vault_y.to_account_info(),
            ),
        };
        let cpi_program = self.token_program.key();
        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.user.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        transfer(cpi_ctx, amount)
    }
}
