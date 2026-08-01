use anchor_lang::prelude::*;

declare_id!("DoYwUP9Ffnvq1UYTnywWKSRYdGgA3X4H74GZj39nLGYW");

#[program]
pub mod friend_bet_v2 {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[account]
pub strut Bet{
    pub id: u64
    pub creator: Pubkey,
    pub ops: List<Pubkey>,
    pub amount: u64,
    pub resolver: Pubkey,
    pub deadline: u64,
    pub status: BetStatus
    pub outcome: Option<Outcome>
    pub bump: u8
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum BetStatus{
    Created,
    Accepted,
    Resolved,
    Cancelled
}