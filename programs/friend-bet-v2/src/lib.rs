use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};


declare_id!("DoYwUP9Ffnvq1UYTnywWKSRYdGgA3X4H74GZj39nLGYW");

#[program]
pub mod friend_bet_v2 {
    use super::*;

    pub fn create_bet(ctx: Context<CreateBet>, init_stake: u64, deadline: i64) -> Result<()> {
        let creator_key =  ctx.accounts.signer.key();

        //validation
        require!(init_stake > 0, MyError::InvalidAmount);
        let now = Clock::get()?.unix_timestamp;
        require!(deadline > (now + 3600), MyError::InvalidDeadline);


        //Setting up a new Bet
        ctx.accounts.new_bet.init_stake = init_stake;
        ctx.accounts.new_bet.deadline = deadline;
        ctx.accounts.new_bet.creator = creator_key;

        //Setting up a new Vault
        ctx.accounts.vault.creator = creator_key;
        ctx.accounts.vault.id = ctx.accounts.new_creator.next_bet_id;
        //transfer creator's stake to vault
        
        let cpi_accounts = Transfer {
            from: ctx.accounts.signer.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            cpi_accounts,
        );
        transfer(cpi_ctx, init_stake)?;

        ctx.accounts.vault.amount +=  ctx.accounts.new_bet.init_stake;


        //Setting Up a New Creator Profile
        ctx.accounts.new_creator.creator = creator_key;

        //Link Bet id to creators next id
        ctx.accounts.new_bet.id = ctx.accounts.new_creator.next_bet_id;

        //Status updates
        ctx.accounts.new_creator.next_bet_id += 1;
        ctx.accounts.new_bet.status = BetStatus::Created;
        
        ctx.accounts.new_bet.bump = ctx.bumps.new_bet;
        ctx.accounts.new_creator.bump = ctx.bumps.new_creator;
        ctx.accounts.vault.bump = ctx.bumps.vault;


        msg!("Bet Created!");
        Ok(())
    }
}

#[derive(Accounts)]
pub struct CreateBet<'info> {
    //Since new_bet's seeds depened on new creator, new creator must get created first

    #[account(
        init_if_needed,
        payer = signer,
        space = 49,
        seeds = [b"creator", signer.key().as_ref()],
        bump
    )]
    pub new_creator: Account<'info,CreatorProfile>,


    #[account(
        init,
        payer = signer,
        space = 100,
        seeds = [b"bet", signer.key().as_ref(), &new_creator.next_bet_id.to_le_bytes()],
        bump
    )]
    pub new_bet: Account<'info,BetAccount>,

    #[account(
        init,
        payer = signer,
        space = 65,
        seeds = [b"vault", signer.key().as_ref(), &new_creator.next_bet_id.to_le_bytes()],
        bump
    )]
    pub vault: Account<'info,Vault>,


    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,

}

// PDA seeds: [creator.key(), id.to_le_bytes()] + bet prefix
// size: 8 (discriminator) + 8 (id) + 32 (creator) + 8 (amount) + 32 (resolver) + 8 (deadline) + 1 (status) + 2 (outcome) + 1 (bump) = 100 bytes
#[account]
pub struct BetAccount{
    pub id: u64,
    pub creator: Pubkey,
    pub init_stake: u64,
    pub resolver: Pubkey,
    pub deadline: i64,
    pub status: BetStatus,
    pub outcome: Option<Outcome>,
    pub bump: u8
}

// PDA seeds: [Creator.key() + bet_id.to_le_bytes()] + bet prefix
// size: 8 (discriminator) + 8 (bet_id) + 32 (creator) + 8 (amount) + 8 (deadline) + 1 (bump) = 65 bytes
#[account]
pub struct Vault{
    pub id: u64, //same as bet id
    pub creator: Pubkey,
    pub amount: u64,
    pub deadline: i64,
    pub bump: u8
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum BetStatus{
    Created,
    Accepted,
    Resolved,
    Cancelled
}

#[error_code]
pub enum MyError{
    InvalidAmount,
    InvalidDeadline
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum Outcome{
    CreatorWon,
    OpponentWon,
    Draw
}

// PDA seeds: [creator.key()] + creator prefix
// size: 8 (discriminator) + 32 (creator) + 8 (next_bet_id) + 1 (bump) = 49 bytes
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
    pub deadline: i64,
    pub stake: u64,
    pub bet: Pubkey, //points to Bet participating in
    pub bump: u8
}