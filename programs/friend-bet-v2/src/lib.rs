use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};


declare_id!("DoYwUP9Ffnvq1UYTnywWKSRYdGgA3X4H74GZj39nLGYW");

#[program]
pub mod friend_bet_v2 {

use super::*;

    pub fn create_bet(ctx: Context<CreateBet>, init_stake: u64, deadline: i64, choice: u8, sides: u8, resolver:Pubkey) -> Result<()> {
        let creator_key =  ctx.accounts.signer.key();
        let bet_id = ctx.accounts.new_creator.next_bet_id;

        //validation
        require!(init_stake > 0, MyError::InvalidAmount);
        let now = Clock::get()?.unix_timestamp;
        require!(deadline > (now + 3600), MyError::InvalidDeadline); //deadline at least one hour away
        require!(sides < 5, MyError::InvalidSides);
        require!(choice < sides, MyError::InvalidChoice);

        //Setting up a new Bet
        ctx.accounts.new_bet.init_stake = init_stake;
        ctx.accounts.new_bet.deadline = deadline;
        ctx.accounts.new_bet.creator = creator_key;
        ctx.accounts.new_bet.sides = sides;

        //Setting up a new Vault
        ctx.accounts.vault.creator = creator_key;
        ctx.accounts.vault.id = bet_id;
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


        //Setting Up a New Creator Profile and Creator's Participant
        ctx.accounts.new_creator.creator = creator_key;
        ctx.accounts.new_particip.owner = creator_key;
        ctx.accounts.new_particip.deadline = deadline;
        ctx.accounts.new_particip.stake = init_stake;
        ctx.accounts.new_particip.bet = ctx.accounts.new_bet.key();

        ctx.accounts.new_particip.choice = choice;

        //Link Bet id to creators next id
        ctx.accounts.new_bet.id = bet_id;

        //Status updates & resolution
        ctx.accounts.new_creator.next_bet_id += 1;
        ctx.accounts.new_bet.status = BetStatus::Created;
        ctx.accounts.new_bet.resolver = resolver; //bet to be resolved by creator

        
        ctx.accounts.new_bet.bump = ctx.bumps.new_bet;
        ctx.accounts.new_creator.bump = ctx.bumps.new_creator;
        ctx.accounts.vault.bump = ctx.bumps.vault;
        ctx.accounts.new_particip.bump = ctx.bumps.new_particip;


        msg!("Bet Created!");
        Ok(())
    }

    //Consult Resolver and Update Winner
    pub fn resolve_bet(ctx:Context<ResolveBet>, winner: u8) -> Result<()> {
        let resolver_key = ctx.accounts.bet.resolver;
        let now = Clock::get()?.unix_timestamp;

        //Validations
        require!(now > ctx.accounts.bet.deadline, MyError::InvalidDeadline);
        require!(ctx.accounts.signer.key() == resolver_key, MyError::UnverifiedSigner);
        require!(ctx.accounts.bet.sides > winner, MyError::InvalidChoice);
        require!(ctx.accounts.bet.status == BetStatus::Accepted, MyError::InvalidStatus);

        
        //Update Winner
        ctx.accounts.bet.winning_choice = Some(winner);

        //Status updates
        ctx.accounts.bet.status = BetStatus::Resolved;
        msg!("Bet Resolved!");
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
        space = 101,
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

    #[account(
        init,
        payer = signer,
        space = 90,
        seeds = [b"particip", signer.key().as_ref(), &new_creator.next_bet_id.to_le_bytes()], //creator is signer so its particp acc owner is signer
        bump
    )]
    pub new_particip: Account<'info,Participant>,


    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,

}

#[derive(Accounts)]
pub struct ResolveBet<'info> {

    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub bet: Account<'info, BetAccount>
}

// PDA seeds: [creator.key(), id.to_le_bytes()] + bet prefix
// size: 8 (discriminator) + 8 (id) + 32 (creator) + 8 (init_stake) + 32 (resolver) + 8 (deadline) + 1 (status) + 1 (sides) + 1 (bump) + 2 (winning choice)= 101 bytes
#[account]
pub struct BetAccount{
    pub id: u64,
    pub creator: Pubkey,
    pub init_stake: u64,
    pub resolver: Pubkey,
    pub deadline: i64,
    pub status: BetStatus,
    pub sides: u8,
    pub bump: u8,
    pub winning_choice: Option<u8>
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

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum BetStatus{
    Created,
    Accepted,
    Resolved,
    Cancelled
}

#[error_code]
pub enum MyError{
    InvalidAmount,
    InvalidDeadline,
    InvalidChoice,
    InvalidSides,
    UnverifiedSigner,
    InvalidStatus
}

// PDA seeds: [creator.key()] + creator prefix
// size: 8 (discriminator) + 32 (creator) + 8 (next_bet_id) + 1 (bump) = 49 bytes
#[account]
pub struct CreatorProfile {
    pub creator: Pubkey,
    pub next_bet_id: u64,
    pub bump: u8,
}

// PDA seeds: [b"particip", owner.key(), bet_id.to_le_bytes()]
// size: 8 (discriminator) + 32 (owner) + 8 (deadline) + 8 (stake) + 32 (bet) + 1 (bump) + 1 (choice) = 90 bytes
#[account]
pub struct Participant{
    pub owner: Pubkey,
    pub deadline: i64,
    pub stake: u64,
    pub bet: Pubkey,
    pub bump: u8,
    pub choice: u8
}