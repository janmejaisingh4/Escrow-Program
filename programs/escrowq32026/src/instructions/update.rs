use anchor_lang::prelude::*;

use crate::{Escrow, ESCROW_SEED};

#[derive(Accounts)]
#[instruction(seed: u64)]
pub struct Update<'info> {
	pub maker: Signer<'info>,
	#[account(
		mut,
		has_one = maker,
		seeds = [ESCROW_SEED, maker.key().as_ref(), seed.to_le_bytes().as_ref()],
		bump = escrow.bump,
	)]
	pub escrow: Account<'info, Escrow>,
}

impl<'info> Update<'info> {
	pub fn update(&mut self, receive: u64, expiration: i64) -> Result<()> {
		self.escrow.receive = receive;
		self.escrow.expiration = expiration;
		Ok(())
	}
}
