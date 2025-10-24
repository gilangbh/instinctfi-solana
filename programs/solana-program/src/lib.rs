use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("6EYowRZgeA51JkwPJ5R1wnhxTYHYumnGYrNZwcGegCnc");

#[program]
pub mod instinct_trading {
    use super::*;

    /// Initialize the platform (one-time setup)
    pub fn initialize_platform(
        ctx: Context<InitializePlatform>,
        platform_fee_bps: u16, // Fee in basis points (e.g., 1500 = 15%)
    ) -> Result<()> {
        require!(platform_fee_bps <= 10000, ErrorCode::InvalidFee);
        
        let platform = &mut ctx.accounts.platform;
        platform.authority = ctx.accounts.authority.key();
        platform.platform_fee_bps = platform_fee_bps;
        platform.total_runs = 0;
        platform.is_paused = false;
        platform.bump = ctx.bumps.platform;

        msg!("Platform initialized with {}% fee", platform_fee_bps as f64 / 100.0);
        Ok(())
    }

    /// Create vault for a run (must be called before users can deposit)
    pub fn create_run_vault(
        ctx: Context<CreateRunVault>,
        run_id: u64,
    ) -> Result<()> {
        msg!("Vault created for run #{}", run_id);
        Ok(())
    }

    /// Create a new trading run
    pub fn create_run(
        ctx: Context<CreateRun>,
        run_id: u64,
        min_deposit: u64,
        max_deposit: u64,
        max_participants: u16,
    ) -> Result<()> {
        require!(!ctx.accounts.platform.is_paused, ErrorCode::PlatformPaused);
        require!(min_deposit > 0, ErrorCode::InvalidDepositAmount);
        require!(max_deposit >= min_deposit, ErrorCode::InvalidDepositAmount);
        require!(max_participants > 0, ErrorCode::InvalidParticipantLimit);

        let run = &mut ctx.accounts.run;
        run.run_id = run_id;
        run.authority = ctx.accounts.platform.authority;
        run.status = RunStatus::Waiting;
        run.total_deposited = 0;
        run.final_balance = 0;
        run.participant_count = 0;
        run.min_deposit = min_deposit;
        run.max_deposit = max_deposit;
        run.max_participants = max_participants;
        run.created_at = Clock::get()?.unix_timestamp;
        run.started_at = 0;
        run.ended_at = 0;
        run.bump = ctx.bumps.run;

        let platform = &mut ctx.accounts.platform;
        platform.total_runs += 1;

        msg!("Run #{} created - Min: {} Max: {} Participants: {}", 
            run_id, min_deposit, max_deposit, max_participants);
        Ok(())
    }

    /// User deposits USDC to join a run
    pub fn deposit(
        ctx: Context<Deposit>,
        run_id: u64,
        amount: u64,
    ) -> Result<()> {
        let run = &mut ctx.accounts.run;
        
        // Validations
        require!(!ctx.accounts.platform.is_paused, ErrorCode::PlatformPaused);
        require!(run.status == RunStatus::Waiting, ErrorCode::RunNotInWaitingPhase);
        require!(amount >= run.min_deposit, ErrorCode::DepositTooLow);
        require!(amount <= run.max_deposit, ErrorCode::DepositTooHigh);
        require!(run.participant_count < run.max_participants, ErrorCode::RunFull);

        // Transfer USDC from user to run vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.run_vault.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        // Update user participation record
        let participation = &mut ctx.accounts.user_participation;
        participation.user = ctx.accounts.user.key();
        participation.run_id = run_id;
        participation.deposit_amount = amount;
        participation.final_share = 0;
        participation.withdrawn = false;
        participation.correct_votes = 0;
        participation.total_votes = 0;
        participation.bump = ctx.bumps.user_participation;

        // Update run totals
        run.total_deposited += amount;
        run.participant_count += 1;

        msg!("User {} deposited {} USDC to run #{}", 
            ctx.accounts.user.key(), amount, run_id);
        Ok(())
    }

    /// Start a run (called by backend authority)
    pub fn start_run(
        ctx: Context<ManageRun>,
        run_id: u64,
    ) -> Result<()> {
        let run = &mut ctx.accounts.run;
        
        require!(run.status == RunStatus::Waiting, ErrorCode::InvalidRunStatus);
        require!(run.participant_count > 0, ErrorCode::NoParticipants);

        run.status = RunStatus::Active;
        run.started_at = Clock::get()?.unix_timestamp;

        msg!("Run #{} started with {} participants and {} USDC", 
            run_id, run.participant_count, run.total_deposited);
        Ok(())
    }

    /// Settle a run with final P/L (called by backend authority after trading ends)
    pub fn settle_run(
        ctx: Context<SettleRun>,
        run_id: u64,
        final_balance: u64,
        participant_shares: Vec<ParticipantShare>,
    ) -> Result<()> {
        let run = &mut ctx.accounts.run;
        
        require!(run.status == RunStatus::Active, ErrorCode::InvalidRunStatus);
        require!(participant_shares.len() == run.participant_count as usize, ErrorCode::InvalidSharesCount);

        // Verify current vault balance matches reported final_balance
        let vault_balance = ctx.accounts.run_vault.amount;
        require!(vault_balance == final_balance, ErrorCode::VaultBalanceMismatch);

        run.status = RunStatus::Settled;
        run.final_balance = final_balance;
        run.ended_at = Clock::get()?.unix_timestamp;

        // Store participant shares for withdrawal
        // Note: In production, you'd want to store this data in separate accounts
        // For MVP, we'll handle distribution through the withdraw instruction

        let profit = if final_balance > run.total_deposited {
            final_balance - run.total_deposited
        } else {
            0
        };

        msg!("Run #{} settled - Initial: {} Final: {} P/L: {}{}", 
            run_id, 
            run.total_deposited, 
            final_balance,
            if profit > 0 { "+" } else { "" },
            profit as i64
        );
        
        Ok(())
    }

    /// Withdraw user's share after run settlement
    pub fn withdraw(
        ctx: Context<Withdraw>,
        run_id: u64,
    ) -> Result<()> {
        let run = &ctx.accounts.run;
        let participation = &mut ctx.accounts.user_participation;

        require!(run.status == RunStatus::Settled, ErrorCode::RunNotSettled);
        require!(!participation.withdrawn, ErrorCode::AlreadyWithdrawn);

        // Calculate user's share
        // Base share = (user_deposit / total_deposited) * final_balance
        // Bonus share = correct_votes * 1% additional
        let base_share_numerator = (participation.deposit_amount as u128)
            .checked_mul(run.final_balance as u128)
            .unwrap();
        let mut user_share = (base_share_numerator / run.total_deposited as u128) as u64;

        // Add bonus for correct votes (max 12% bonus if all 12 votes correct)
        let correct_vote_bonus_bps = participation.correct_votes as u64 * 100; // 1% per correct vote
        let bonus = (user_share as u128 * correct_vote_bonus_bps as u128 / 10000) as u64;
        user_share += bonus;

        // Ensure we don't withdraw more than vault has
        require!(user_share <= ctx.accounts.run_vault.amount, ErrorCode::InsufficientVaultFunds);

        // Transfer USDC from vault to user
        let run_id_bytes = run.run_id.to_le_bytes();
        let run_seeds = &[
            b"run",
            run_id_bytes.as_ref(),
            &[run.bump],
        ];
        let signer = &[&run_seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.run_vault.to_account_info(),
            to: ctx.accounts.user_token_account.to_account_info(),
            authority: ctx.accounts.run.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, user_share)?;

        participation.final_share = user_share;
        participation.withdrawn = true;

        msg!("User {} withdrew {} USDC from run #{}", 
            ctx.accounts.user.key(), user_share, run_id);
        Ok(())
    }

    /// Update user's vote statistics (called by backend after each voting round)
    pub fn update_vote_stats(
        ctx: Context<UpdateVoteStats>,
        run_id: u64,
        user_pubkey: Pubkey,
        correct_votes: u8,
        total_votes: u8,
    ) -> Result<()> {
        let participation = &mut ctx.accounts.user_participation;
        
        require!(ctx.accounts.run.status == RunStatus::Active, ErrorCode::InvalidRunStatus);
        
        participation.correct_votes = correct_votes;
        participation.total_votes = total_votes;

        Ok(())
    }

    /// Emergency pause (admin only)
    pub fn pause_platform(ctx: Context<AdminAction>) -> Result<()> {
        ctx.accounts.platform.is_paused = true;
        msg!("Platform paused by authority");
        Ok(())
    }

    /// Unpause platform (admin only)
    pub fn unpause_platform(ctx: Context<AdminAction>) -> Result<()> {
        ctx.accounts.platform.is_paused = false;
        msg!("Platform unpaused by authority");
        Ok(())
    }

    /// Emergency withdraw (admin only - for stuck funds)
    pub fn emergency_withdraw(
        ctx: Context<EmergencyWithdraw>,
        run_id: u64,
        amount: u64,
    ) -> Result<()> {
        require!(ctx.accounts.platform.is_paused, ErrorCode::PlatformNotPaused);

        let run = &ctx.accounts.run;
        let run_id_bytes = run.run_id.to_le_bytes();
        let run_seeds = &[
            b"run",
            run_id_bytes.as_ref(),
            &[run.bump],
        ];
        let signer = &[&run_seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.run_vault.to_account_info(),
            to: ctx.accounts.destination_token_account.to_account_info(),
            authority: ctx.accounts.run.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;

        msg!("Emergency withdraw: {} USDC from run #{}", amount, run_id);
        Ok(())
    }
}

// ============================================================================
// Account Structures
// ============================================================================

#[account]
pub struct Platform {
    pub authority: Pubkey,           // Platform admin
    pub platform_fee_bps: u16,       // Fee in basis points (1500 = 15%)
    pub total_runs: u64,             // Total runs created
    pub is_paused: bool,             // Emergency pause flag
    pub bump: u8,                    // PDA bump
}

impl Platform {
    pub const LEN: usize = 8 + 32 + 2 + 8 + 1 + 1;
}

#[account]
pub struct Run {
    pub run_id: u64,                 // Unique run identifier
    pub authority: Pubkey,           // Platform authority
    pub status: RunStatus,           // Current status
    pub total_deposited: u64,        // Total USDC deposited
    pub final_balance: u64,          // Final balance after trading
    pub participant_count: u16,      // Number of participants
    pub min_deposit: u64,            // Minimum deposit (e.g., 10 USDC)
    pub max_deposit: u64,            // Maximum deposit (e.g., 100 USDC)
    pub max_participants: u16,       // Max participants (e.g., 100)
    pub created_at: i64,             // Unix timestamp
    pub started_at: i64,             // Unix timestamp
    pub ended_at: i64,               // Unix timestamp
    pub bump: u8,                    // PDA bump
}

impl Run {
    pub const LEN: usize = 8 + 8 + 32 + 1 + 8 + 8 + 2 + 8 + 8 + 2 + 8 + 8 + 8 + 1;
}

#[account]
pub struct UserParticipation {
    pub user: Pubkey,                // User wallet
    pub run_id: u64,                 // Associated run
    pub deposit_amount: u64,         // Amount deposited
    pub final_share: u64,            // Final share received
    pub withdrawn: bool,             // Withdrawal status
    pub correct_votes: u8,           // Number of correct votes
    pub total_votes: u8,             // Total votes cast
    pub bump: u8,                    // PDA bump
}

impl UserParticipation {
    pub const LEN: usize = 8 + 32 + 8 + 8 + 8 + 1 + 1 + 1 + 1;
}

// ============================================================================
// Enums
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum RunStatus {
    Waiting,   // Accepting deposits
    Active,    // Trading in progress
    Settled,   // Trading ended, ready for withdrawals
}

// ============================================================================
// Context Structs
// ============================================================================

#[derive(Accounts)]
pub struct InitializePlatform<'info> {
    #[account(
        init,
        payer = authority,
        space = Platform::LEN,
        seeds = [b"platform"],
        bump
    )]
    pub platform: Account<'info, Platform>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(run_id: u64)]
pub struct CreateRun<'info> {
    #[account(
        mut,
        seeds = [b"platform"],
        bump = platform.bump
    )]
    pub platform: Account<'info, Platform>,
    
    #[account(
        init,
        payer = authority,
        space = Run::LEN,
        seeds = [b"run", run_id.to_le_bytes().as_ref()],
        bump
    )]
    pub run: Account<'info, Run>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(run_id: u64)]
pub struct CreateRunVault<'info> {
    #[account(
        seeds = [b"run", run_id.to_le_bytes().as_ref()],
        bump = run.bump
    )]
    pub run: Account<'info, Run>,
    
    #[account(
        init,
        payer = payer,
        token::mint = usdc_mint,
        token::authority = run,
        seeds = [b"vault", run_id.to_le_bytes().as_ref()],
        bump
    )]
    pub run_vault: Account<'info, TokenAccount>,
    
    pub usdc_mint: Account<'info, token::Mint>,
    
    #[account(mut)]
    pub payer: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(run_id: u64)]
pub struct Deposit<'info> {
    #[account(seeds = [b"platform"], bump = platform.bump)]
    pub platform: Account<'info, Platform>,
    
    #[account(
        mut,
        seeds = [b"run", run_id.to_le_bytes().as_ref()],
        bump = run.bump
    )]
    pub run: Account<'info, Run>,
    
    #[account(
        init,
        payer = user,
        space = UserParticipation::LEN,
        seeds = [b"participation", run_id.to_le_bytes().as_ref(), user.key().as_ref()],
        bump
    )]
    pub user_participation: Account<'info, UserParticipation>,
    
    #[account(
        mut,
        seeds = [b"vault", run_id.to_le_bytes().as_ref()],
        bump
    )]
    pub run_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    
    pub usdc_mint: Account<'info, token::Mint>,
    
    #[account(mut)]
    pub user: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(run_id: u64)]
pub struct ManageRun<'info> {
    #[account(seeds = [b"platform"], bump = platform.bump)]
    pub platform: Account<'info, Platform>,
    
    #[account(
        mut,
        seeds = [b"run", run_id.to_le_bytes().as_ref()],
        bump = run.bump,
        has_one = authority
    )]
    pub run: Account<'info, Run>,
    
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(run_id: u64)]
pub struct SettleRun<'info> {
    #[account(seeds = [b"platform"], bump = platform.bump)]
    pub platform: Account<'info, Platform>,
    
    #[account(
        mut,
        seeds = [b"run", run_id.to_le_bytes().as_ref()],
        bump = run.bump,
        has_one = authority
    )]
    pub run: Account<'info, Run>,
    
    #[account(
        seeds = [b"vault", run_id.to_le_bytes().as_ref()],
        bump
    )]
    pub run_vault: Account<'info, TokenAccount>,
    
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(run_id: u64)]
pub struct Withdraw<'info> {
    #[account(
        seeds = [b"run", run_id.to_le_bytes().as_ref()],
        bump = run.bump
    )]
    pub run: Account<'info, Run>,
    
    #[account(
        mut,
        seeds = [b"participation", run_id.to_le_bytes().as_ref(), user.key().as_ref()],
        bump = user_participation.bump
    )]
    pub user_participation: Account<'info, UserParticipation>,
    
    #[account(
        mut,
        seeds = [b"vault", run_id.to_le_bytes().as_ref()],
        bump
    )]
    pub run_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    
    pub user: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(run_id: u64, user_pubkey: Pubkey)]
pub struct UpdateVoteStats<'info> {
    #[account(seeds = [b"platform"], bump = platform.bump)]
    pub platform: Account<'info, Platform>,
    
    #[account(
        seeds = [b"run", run_id.to_le_bytes().as_ref()],
        bump = run.bump,
        has_one = authority
    )]
    pub run: Account<'info, Run>,
    
    #[account(
        mut,
        seeds = [b"participation", run_id.to_le_bytes().as_ref(), user_pubkey.as_ref()],
        bump = user_participation.bump
    )]
    pub user_participation: Account<'info, UserParticipation>,
    
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(
        mut,
        seeds = [b"platform"],
        bump = platform.bump,
        has_one = authority
    )]
    pub platform: Account<'info, Platform>,
    
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(run_id: u64)]
pub struct EmergencyWithdraw<'info> {
    #[account(seeds = [b"platform"], bump = platform.bump, has_one = authority)]
    pub platform: Account<'info, Platform>,
    
    #[account(
        seeds = [b"run", run_id.to_le_bytes().as_ref()],
        bump = run.bump
    )]
    pub run: Account<'info, Run>,
    
    #[account(
        mut,
        seeds = [b"vault", run_id.to_le_bytes().as_ref()],
        bump
    )]
    pub run_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub destination_token_account: Account<'info, TokenAccount>,
    
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// ============================================================================
// Helper Structs
// ============================================================================

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct ParticipantShare {
    pub user: Pubkey,
    pub share_amount: u64,
}

// ============================================================================
// Error Codes
// ============================================================================

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid fee percentage")]
    InvalidFee,
    
    #[msg("Platform is paused")]
    PlatformPaused,
    
    #[msg("Platform must be paused for this operation")]
    PlatformNotPaused,
    
    #[msg("Invalid deposit amount")]
    InvalidDepositAmount,
    
    #[msg("Invalid participant limit")]
    InvalidParticipantLimit,
    
    #[msg("Run is not in waiting phase")]
    RunNotInWaitingPhase,
    
    #[msg("Deposit amount is below minimum")]
    DepositTooLow,
    
    #[msg("Deposit amount exceeds maximum")]
    DepositTooHigh,
    
    #[msg("Run is full")]
    RunFull,
    
    #[msg("Invalid run status for this operation")]
    InvalidRunStatus,
    
    #[msg("Run has no participants")]
    NoParticipants,
    
    #[msg("Invalid number of participant shares")]
    InvalidSharesCount,
    
    #[msg("Vault balance does not match reported final balance")]
    VaultBalanceMismatch,
    
    #[msg("Run is not settled yet")]
    RunNotSettled,
    
    #[msg("User has already withdrawn")]
    AlreadyWithdrawn,
    
    #[msg("Insufficient funds in vault")]
    InsufficientVaultFunds,
}
