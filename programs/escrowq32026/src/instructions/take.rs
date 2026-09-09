use crate::{error::ErrorCode as EscrowError, Escrow, ESCROW_SEED};
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token_interface::{
    close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
    TransferChecked,
};


//taker - signer, taker ata a, taker ata b
//maker - system account, maker ata a, escrow, vault
//mint a and mint b are the same token, but different accounts
//taker_ata_a - which will recieve funds from vault, and 
//taker_ata_b - which will send funds to maker_ata_b
//maker_ata_b - which will receive funds from taker
//associative token program, token program, system program
#[derive(Accounts)]
pub struct Take<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,
    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(
        mint::token_program = token_program,
    )]
    pub mint_a: Box<InterfaceAccount<'info, Mint>>,
    #[account(
        mint::token_program = token_program,
    )]
    pub mint_b: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_a,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_a: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        associated_token::mint = mint_b,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,
    
    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = mint_b,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_ata_b: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        close = maker,
        seeds = [ESCROW_SEED, escrow.maker.as_ref(), escrow.seed.to_le_bytes().as_ref()],
        bump = escrow.bump,
        has_one = mint_a,
        has_one = mint_b,
        has_one = maker,
    )]
    pub escrow: Box<Account<'info, Escrow>>,

    #[account(
        mut,
        associated_token::mint = mint_a,
        associated_token::authority = escrow,
        associated_token::token_program = token_program,
    )]
    pub vault: Box<InterfaceAccount<'info, TokenAccount>>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl <'info>Take<'info>{
    //tranfer the tokens from taker to maker
    pub fn deposit(&mut self) -> Result<()>{
        require!(
            Clock::get()?.unix_timestamp < self.escrow.expiration,
            EscrowError::EscrowExpired
        );

        let cpl_accounts : TransferChecked<'_> = TransferChecked{
            from: self.taker_ata_b.to_account_info(),
            mint: self.mint_b.to_account_info(),
            to: self.maker_ata_b.to_account_info(),
            authority: self.taker.to_account_info(),
        };

        let cpl_ctx = CpiContext::new(self.token_program.key(), cpl_accounts);

        // transfer from taker_ata_b to maker_ata_b
        transfer_checked(cpl_ctx, self.escrow.receive, self.mint_b.decimals)?;
        Ok(())
    }

    // withdraw the tokens from vault to taker and close the escrow account 
    pub fn withdraw_and_close_vault(&mut self) -> Result<()>{

        let cpl_accounts : TransferChecked<'_> = TransferChecked{
            from: self.vault.to_account_info(),
            mint: self.mint_a.to_account_info(),
            to: self.taker_ata_a.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let signer_seeds : [&[&[u8]]; 1] = [&[
            ESCROW_SEED,
            self.escrow.maker.as_ref(),
            &self.escrow.seed.to_le_bytes()[..],
            &[self.escrow.bump],
        ]];

        let cpl_ctx = CpiContext::new_with_signer(
            self.token_program.key(),
            cpl_accounts,
            &signer_seeds,
        );

        transfer_checked(cpl_ctx, self.vault.amount, self.mint_a.decimals)?;

        // close the vault account and send the rent back to maker
        let cpl_accounts = CloseAccount{
            account: self.vault.to_account_info(),
            destination: self.maker.to_account_info(),
            authority: self.escrow.to_account_info(),
        };

        let cpl_ctx = CpiContext::new_with_signer(
            self.token_program.key(),
            cpl_accounts,
            &signer_seeds,
        );

        close_account(cpl_ctx)?;
        Ok(())
    }
}