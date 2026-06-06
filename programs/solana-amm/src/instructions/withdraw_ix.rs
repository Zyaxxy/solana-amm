use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{transfer, Transfer};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use solana_instructions_sysvar::load_instruction_at_checked;

use crate::error::AmmError;
use crate::state::Config;
use constant_product_curve::ConstantProduct;

const LP_BURN_INSTRUCTION_SIZE: usize = 16; // 8 bytes for the instruction discriminator + 8 bytes for the amount
#[derive(Accounts)]
pub struct WithdrawIx<'info> {
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
        mut,
        associated_token::mint = mint_lp,
        associated_token::authority = user,
        associated_token::token_program = token_program,
    )]
    pub user_lp: InterfaceAccount<'info, TokenAccount>,
    /// CHECK: validated manually against the well-known instructions sysvar key
    pub instruction_sysvar: UncheckedAccount<'info>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> WithdrawIx<'info> {
    pub fn withdraw_ix(&mut self, amount: u64, min_x: u64, min_y: u64) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require_neq!(amount, 0, AmmError::InvalidAmount);

        self.verify_burn(amount)?;

        let pre_burn_supply = self.mint_lp.supply.checked_add(amount).ok_or(AmmError::Overflow)?; 

        let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
            self.vault_x.amount,
            self.vault_y.amount,
            pre_burn_supply,
            amount,
            6,
        )
        .unwrap();

        require!(
            amounts.x >= min_x && amounts.y >= min_y,
            AmmError::SlippageExceeded
        );

        let (x, y) = (amounts.x, amounts.y);

        self.withdraw_tokens(true, x)?;
        self.withdraw_tokens(false, y)?;
        Ok(())
    }

    fn verify_burn(&self, amount: u64) -> Result<()> {
        let burn_ix = load_instruction_at_checked(
            0,
            &self.instruction_sysvar.to_account_info(),
        )
        .map_err(|_| AmmError::InvalidBurnInstruction)?;

        // Verify the burn instruction
        require_keys_eq!(burn_ix.program_id, crate::ID, AmmError::InvalidBurnInstruction);
        require_eq!(burn_ix.accounts.len(), 5, AmmError::InvalidBurnInstruction);
        require_eq!(
            burn_ix.data.len(),
            LP_BURN_INSTRUCTION_SIZE,
            AmmError::InvalidBurnInstruction
        );
        
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&burn_ix.data[8..16]);
        let burn_amount = u64::from_le_bytes(amount_bytes);

        require_eq!(burn_amount, amount, AmmError::InvalidAmount);
        let burn_user = &burn_ix.accounts[0];
        let burn_config = &burn_ix.accounts[1];
        let burn_mint_lp = &burn_ix.accounts[2];
        let burn_user_lp = &burn_ix.accounts[3];
        let burn_token_program = &burn_ix.accounts[4];
        

        require_keys_eq!(burn_user.pubkey, self.user.key(), AmmError::InvalidBurnInstruction);
        require_keys_eq!(burn_config.pubkey, self.config.key(), AmmError::InvalidBurnInstruction);
        require_keys_eq!(burn_mint_lp.pubkey, self.mint_lp.key(), AmmError::InvalidBurnInstruction);
        require_keys_eq!(burn_user_lp.pubkey, self.user_lp.key(), AmmError::InvalidBurnInstruction);
        require_keys_eq!(burn_token_program.pubkey, self.token_program.key(), AmmError::InvalidBurnInstruction);
        Ok(())
    }

    pub fn withdraw_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            ),
            false => (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            ),
        };
        let cpi_program = self.token_program.key();
        let cpi_accounts = Transfer {
            from,
            to,
            authority: self.config.to_account_info(),
        };
        let configseed = self.config.seed.to_le_bytes();
        let seeds= &[
            b"config",
            configseed.as_ref(),
            &[self.config.config_bump],
        ];
        let signer_seeds = &[seeds.as_ref()];
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        transfer(cpi_ctx, amount)
    }
}
