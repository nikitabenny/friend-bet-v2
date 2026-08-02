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

// PDA seeds: [creator.key(), id.to_le_bytes()]
#[account]
pub struct Bet{
    pub id: u64,
    pub creator: Pubkey,
    pub amount: u64,
    pub resolver: Pubkey,
    pub deadline: u64,
    pub status: BetStatus,
    pub outcome: Option<Outcome>,
    pub bump: u8
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum BetStatus{
    Created,
    Accepted,
    Resolved,
    Cancelled
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum Outcome{
    CreatorWon,
    OpponentWon,
    Draw
}

// PDA seeds: [creator.key()]
#[account]
pub struct CreatorProfile {
    pub creator: Pubkey,
    pub next_bet_id: u64,
    pub bump: u8,
}

// PDA seeds: [owner.key(), bet.key()]
#[account]
pub struct Participant{
    pub owner: Pubkey, 
    pub deadline: u64,
    pub stake: u64,
    pub bet: Pubkey //points to Bet participating in
}