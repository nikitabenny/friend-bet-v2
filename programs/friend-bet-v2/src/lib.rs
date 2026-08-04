use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};


declare_id!("DoYwUP9Ffnvq1UYTnywWKSRYdGgA3X4H74GZj39nLGYW");

#[program]
pub mod friend_bet_v2 {
use super::*;

    pub fn create_bet(ctx: Context<CreateBet>, init_stake: u64, deadline: i64, choice: u64, sides: u64, resolver:Pubkey) -> Result<()> {
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
        ctx.accounts.new_particip.stake = init_stake;
        ctx.accounts.new_particip.claimed = false;
        ctx.accounts.new_particip.bet = ctx.accounts.new_bet.key();

        ctx.accounts.new_particip.choice = choice;

        //Link Bet id to creators next id
        ctx.accounts.new_bet.id = bet_id;

        //Status updates & resolution
        ctx.accounts.new_creator.next_bet_id += 1;
        ctx.accounts.new_bet.status = BetStatus::Created;
        ctx.accounts.new_bet.resolver = resolver; //bet to be resolved by 3rd party wallet

        //add stake to side's bet
        ctx.accounts.new_bet.side_totals[choice as usize] += init_stake;

        
        ctx.accounts.new_bet.bump = ctx.bumps.new_bet;
        ctx.accounts.new_creator.bump = ctx.bumps.new_creator;
        ctx.accounts.vault.bump = ctx.bumps.vault;
        ctx.accounts.new_particip.bump = ctx.bumps.new_particip;


        msg!("Bet Created!");
        Ok(())
    }

    //Add Participant
    pub fn accept_bet(ctx:Context<AcceptBet>, choice: u64, stake : u64) -> Result<()> {
        //validation 
        require!(stake > 0, MyError::InvalidAmount);
        let now = Clock::get()?.unix_timestamp;

        require!(now < ctx.accounts.bet.deadline, MyError::InvalidDeadline); //deadline is later
        require!(choice < ctx.accounts.bet.sides, MyError::InvalidChoice);
        require!(ctx.accounts.bet.status == BetStatus::Created || ctx.accounts.bet.status == BetStatus::Accepted, MyError::InvalidStatus);
        require!(ctx.accounts.bet.id == ctx.accounts.vault.id, MyError::MismatchedVault);

        //set up participant
        ctx.accounts.particip.choice = choice;
        ctx.accounts.particip.owner = ctx.accounts.signer.key();
        ctx.accounts.particip.stake = stake;
        ctx.accounts.particip.bet =  ctx.accounts.bet.key();
        ctx.accounts.particip.claimed = false;
        ctx.accounts.particip.bump = ctx.bumps.particip;

        //add stake to side's bet
        ctx.accounts.bet.side_totals[choice as usize] += stake;


        //transfer participant's stake to vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.signer.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            cpi_accounts,
        );

        transfer(cpi_ctx, ctx.accounts.particip.stake)?;
        ctx.accounts.vault.amount +=  ctx.accounts.particip.stake;

        ctx.accounts.bet.status = BetStatus::Accepted;

        msg!("Bet Accepted!");
        Ok(())
    }

    //Consult Resolver and Update Winner
    pub fn resolve_bet(ctx:Context<ResolveBet>, winner: u64) -> Result<()> {
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

    pub fn payout(ctx:Context<Payout>) -> Result<()> {
        require!(ctx.accounts.signer.key() == ctx.accounts.participant.owner, MyError::UnverifiedSigner);
        require!(ctx.accounts.participant.bet == ctx.accounts.bet.key(), MyError::MismatchedVault);
        require!(ctx.accounts.bet.id == ctx.accounts.vault.id, MyError::MismatchedVault);
        require!(ctx.accounts.bet.status == BetStatus::Resolved, MyError::InvalidStatus);
        require!(ctx.accounts.participant.claimed == false, MyError::DoubleClaim);
        let winning_choice = ctx.accounts.bet.winning_choice.unwrap();
        require!(ctx.accounts.participant.choice == winning_choice, MyError::LoserClaim);
        let total_winning_stake = ctx.accounts.bet.side_totals[winning_choice as usize];


        let payout: u64 = (ctx.accounts.participant.stake as u128 * ctx.accounts.vault.amount as u128 / total_winning_stake as u128) as u64;

        require!(total_winning_stake > 0, MyError::InvalidStatus);

        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= payout;
        **ctx.accounts.signer.to_account_info().try_borrow_mut_lamports()? += payout;

        ctx.accounts.participant.claimed = true;

        msg!("Claimed {} lamports", payout);

        Ok(())
    }

    pub fn cancel_bet(ctx: Context<CancelBet>) -> Result<()> {
    require!(ctx.accounts.bet.status == BetStatus::Created, MyError::InvalidStatus);
    require!(ctx.accounts.signer.key() == ctx.accounts.bet.creator, MyError::UnverifiedSigner);
    let now = Clock::get()?.unix_timestamp;
    require!(now > ctx.accounts.bet.deadline, MyError::InvalidDeadline);

    let refund = ctx.accounts.vault.amount;

    **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= refund;
    **ctx.accounts.signer.to_account_info().try_borrow_mut_lamports()? += refund;

    ctx.accounts.bet.status = BetStatus::Cancelled;

    msg!("Bet Cancelled, {} lamports refunded", refund);
    Ok(())
}



}

#[derive(Accounts)]
pub struct CancelBet<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub bet: Account<'info, BetAccount>,

    #[account(mut)]
    pub vault: Account<'info, Vault>,
}

#[derive(Accounts)]
pub struct Payout<'info>{
    #[account(mut)]
    pub vault: Account<'info,Vault>,
    #[account(mut)]
    pub participant: Account<'info,Participant>,
    #[account(mut)]
    pub bet: Account<'info,BetAccount>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
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
        space = 155,
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

    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub bet: Account<'info, BetAccount>
}

#[derive(Accounts)]
pub struct AcceptBet<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,

    #[account(mut)]
    pub bet: Account<'info, BetAccount>,

    #[account(
        init,
        payer = signer,
        space = 90,
        seeds = [b"particip", signer.key().as_ref(), &bet.id.to_le_bytes()], 
        bump
    )]
    pub particip: Account<'info, Participant>,

    #[account(mut)]
    pub vault: Account<'info, Vault>,
    pub system_program: Program<'info, System>,
}



// PDA seeds: [creator.key(), id.to_le_bytes()] + bet prefix
// size: 8 (discriminator) + 8 (id) + 32 (creator) + 8 (init_stake) + 32 (resolver) + 8 (deadline) + 1 (status) + 8 (sides) + 1 (bump) + 9 (winning_choice) + 40 (side_totals) = 155 bytes
#[account]
pub struct BetAccount{
    pub id: u64,
    pub creator: Pubkey,
    pub init_stake: u64,
    pub resolver: Pubkey,
    pub deadline: i64,
    pub status: BetStatus,
    pub sides: u64,
    pub bump: u8,
    pub winning_choice: Option<u64>,
    pub side_totals: [u64; 5], // index = choice
}

// PDA seeds: [Creator.key() + bet_id.to_le_bytes()] + vault prefix
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
    Resolved, //outcome determined  
    Complete, //Payout Complete
    Cancelled
}

#[error_code]
pub enum MyError{
    InvalidAmount,
    InvalidDeadline,
    InvalidChoice,
    InvalidSides,
    UnverifiedSigner,
    InvalidStatus,
    MismatchedVault,
    DoubleClaim,
    LoserClaim
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
// size: 8 (discriminator) + 32 (owner) + 8 (stake) + 32 (bet) + 1 (bump) + 8 (choice) + 1 (claimed) = 90 bytes
#[account]
pub struct Participant{
    pub owner: Pubkey,
    pub stake: u64,
    pub bet: Pubkey,
    pub bump: u8,
    pub choice: u64,
    pub claimed: bool
}