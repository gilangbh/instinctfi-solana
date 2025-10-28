# Critical Fixes - Implementation Guide

This document provides detailed code changes to fix the critical issues identified in the security audit.

---

## Fix #1: Implement Platform Fee Collection

### Changes Required

#### 1. Update `Platform` struct to track collected fees

```rust
#[account]
pub struct Platform {
    pub authority: Pubkey,
    pub platform_fee_bps: u16,
    pub total_runs: u64,
    pub is_paused: bool,
    pub bump: u8,
    pub total_fees_collected: u64,      // NEW: Track total fees
    pub platform_fee_vault: Pubkey,      // NEW: Vault to store fees
}

impl Platform {
    pub const LEN: usize = 8 + 32 + 2 + 8 + 1 + 1 + 8 + 32; // Updated size
}
```

#### 2. Add platform fee vault initialization

```rust
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
    
    #[account(
        init,
        payer = authority,
        token::mint = usdc_mint,
        token::authority = platform,
        seeds = [b"platform_fee_vault"],
        bump
    )]
    pub platform_fee_vault: Account<'info, TokenAccount>,
    
    pub usdc_mint: Account<'info, token::Mint>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn initialize_platform(
    ctx: Context<InitializePlatform>,
    platform_fee_bps: u16,
) -> Result<()> {
    require!(platform_fee_bps <= 10000, ErrorCode::InvalidFee);
    
    let platform = &mut ctx.accounts.platform;
    platform.authority = ctx.accounts.authority.key();
    platform.platform_fee_bps = platform_fee_bps;
    platform.total_runs = 0;
    platform.is_paused = false;
    platform.bump = ctx.bumps.platform;
    platform.total_fees_collected = 0;
    platform.platform_fee_vault = ctx.accounts.platform_fee_vault.key();

    Ok(())
}
```

#### 3. Update `Run` struct to track fee

```rust
#[account]
pub struct Run {
    pub run_id: u64,
    pub authority: Pubkey,
    pub status: RunStatus,
    pub total_deposited: u64,
    pub final_balance: u64,
    pub platform_fee_amount: u64,        // NEW: Fee for this run
    pub participant_count: u16,
    pub min_deposit: u64,
    pub max_deposit: u64,
    pub max_participants: u16,
    pub created_at: i64,
    pub started_at: i64,
    pub ended_at: i64,
    pub bump: u8,
}

impl Run {
    pub const LEN: usize = 8 + 8 + 32 + 1 + 8 + 8 + 8 + 2 + 8 + 8 + 2 + 8 + 8 + 8 + 1; // Updated
}
```

#### 4. Modify `settle_run` to calculate and deduct fee

```rust
#[derive(Accounts)]
#[instruction(run_id: u64)]
pub struct SettleRun<'info> {
    #[account(
        mut,
        seeds = [b"platform"],
        bump = platform.bump
    )]
    pub platform: Account<'info, Platform>,
    
    #[account(
        mut,
        seeds = [b"run", run_id.to_le_bytes().as_ref()],
        bump = run.bump,
        has_one = authority
    )]
    pub run: Account<'info, Run>,
    
    #[account(
        mut,
        seeds = [b"vault", run_id.to_le_bytes().as_ref()],
        bump
    )]
    pub run_vault: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"platform_fee_vault"],
        bump
    )]
    pub platform_fee_vault: Account<'info, TokenAccount>,
    
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn settle_run(
    ctx: Context<SettleRun>,
    run_id: u64,
    final_balance: u64,
    participant_shares: Vec<ParticipantShare>,
) -> Result<()> {
    let run = &mut ctx.accounts.run;
    let platform = &mut ctx.accounts.platform;
    
    require!(run.status == RunStatus::Active, ErrorCode::InvalidRunStatus);
    require!(
        participant_shares.len() == run.participant_count as usize,
        ErrorCode::InvalidSharesCount
    );

    // Verify current vault balance matches reported final_balance
    let vault_balance = ctx.accounts.run_vault.amount;
    require!(vault_balance == final_balance, ErrorCode::VaultBalanceMismatch);

    // Calculate platform fee ONLY on profit (not on principal)
    let profit = if final_balance > run.total_deposited {
        final_balance
            .checked_sub(run.total_deposited)
            .ok_or(ErrorCode::ArithmeticOverflow)?
    } else {
        0
    };

    let platform_fee = (profit as u128)
        .checked_mul(platform.platform_fee_bps as u128)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(10000)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        as u64;

    // Transfer platform fee to platform vault
    if platform_fee > 0 {
        let run_id_bytes = run.run_id.to_le_bytes();
        let run_seeds = &[
            b"run",
            run_id_bytes.as_ref(),
            &[run.bump],
        ];
        let signer = &[&run_seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.run_vault.to_account_info(),
            to: ctx.accounts.platform_fee_vault.to_account_info(),
            authority: ctx.accounts.run.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, platform_fee)?;
    }

    // Update run state
    run.status = RunStatus::Settled;
    run.final_balance = final_balance
        .checked_sub(platform_fee)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    run.platform_fee_amount = platform_fee;
    run.ended_at = Clock::get()?.unix_timestamp;

    // Update platform totals
    platform.total_fees_collected = platform.total_fees_collected
        .checked_add(platform_fee)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    msg!(
        "Run #{} settled - Initial: {} Final: {} Fee: {} Available: {}",
        run_id,
        run.total_deposited,
        final_balance,
        platform_fee,
        run.final_balance
    );

    Ok(())
}
```

#### 5. Add platform fee withdrawal function

```rust
#[derive(Accounts)]
pub struct WithdrawPlatformFees<'info> {
    #[account(
        mut,
        seeds = [b"platform"],
        bump = platform.bump,
        has_one = authority
    )]
    pub platform: Account<'info, Platform>,
    
    #[account(
        mut,
        seeds = [b"platform_fee_vault"],
        bump
    )]
    pub platform_fee_vault: Account<'info, TokenAccount>,
    
    #[account(mut)]
    pub destination_token_account: Account<'info, TokenAccount>,
    
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn withdraw_platform_fees(
    ctx: Context<WithdrawPlatformFees>,
    amount: u64,
) -> Result<()> {
    require!(
        amount <= ctx.accounts.platform_fee_vault.amount,
        ErrorCode::InsufficientVaultFunds
    );

    let platform_seeds = &[
        b"platform",
        &[ctx.accounts.platform.bump],
    ];
    let signer = &[&platform_seeds[..]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.platform_fee_vault.to_account_info(),
        to: ctx.accounts.destination_token_account.to_account_info(),
        authority: ctx.accounts.platform.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::transfer(cpi_ctx, amount)?;

    msg!("Platform fees withdrawn: {} USDC", amount);
    Ok(())
}
```

---

## Fix #2: Fix Withdrawal Share Calculation Rounding

### Changes Required

#### 1. Add withdrawal tracking to `Run` struct

```rust
#[account]
pub struct Run {
    // ... existing fields ...
    pub total_withdrawn: u64,            // NEW: Track total withdrawn
    pub withdrawn_count: u16,            // NEW: Count of withdrawals
}
```

#### 2. Implement fair rounding in withdrawal

```rust
pub fn withdraw(
    ctx: Context<Withdraw>,
    run_id: u64,
) -> Result<()> {
    let run = &mut ctx.accounts.run;
    let participation = &mut ctx.accounts.user_participation;

    require!(run.status == RunStatus::Settled, ErrorCode::RunNotSettled);
    require!(!participation.withdrawn, ErrorCode::AlreadyWithdrawn);

    let mut user_share: u64;

    // Check if this is the last withdrawal
    let is_last_user = run.withdrawn_count + 1 == run.participant_count;

    if is_last_user {
        // Last user gets all remaining balance (fixes rounding dust)
        user_share = ctx.accounts.run_vault.amount;
        
        msg!(
            "Last withdrawal - giving remaining vault balance: {}",
            user_share
        );
    } else {
        // Calculate proportional share for non-last users
        let base_share_numerator = (participation.deposit_amount as u128)
            .checked_mul(run.final_balance as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        
        let base_share = base_share_numerator
            .checked_div(run.total_deposited as u128)
            .ok_or(ErrorCode::ArithmeticOverflow)? as u64;

        // Calculate bonus only if there was profit
        if run.final_balance > run.total_deposited {
            // Bonus is on profit share only
            let profit_ratio = run.final_balance
                .checked_sub(run.total_deposited)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
            
            let user_profit_share = (participation.deposit_amount as u128)
                .checked_mul(profit_ratio as u128)
                .ok_or(ErrorCode::ArithmeticOverflow)?
                .checked_div(run.total_deposited as u128)
                .ok_or(ErrorCode::ArithmeticOverflow)? as u64;

            let correct_vote_bonus_bps = (participation.correct_votes as u64)
                .checked_mul(100)
                .ok_or(ErrorCode::ArithmeticOverflow)?; // 1% per vote
            
            let bonus = (user_profit_share as u128)
                .checked_mul(correct_vote_bonus_bps as u128)
                .ok_or(ErrorCode::ArithmeticOverflow)?
                .checked_div(10000)
                .ok_or(ErrorCode::ArithmeticOverflow)? as u64;

            user_share = base_share
                .checked_add(bonus)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
        } else {
            // No bonus on losses
            user_share = base_share;
        }

        // Ensure we don't exceed vault balance
        require!(
            user_share <= ctx.accounts.run_vault.amount,
            ErrorCode::InsufficientVaultFunds
        );
    }

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

    // Update participation
    participation.final_share = user_share;
    participation.withdrawn = true;

    // Update run withdrawal tracking
    run.total_withdrawn = run.total_withdrawn
        .checked_add(user_share)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    run.withdrawn_count = run.withdrawn_count
        .checked_add(1)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    msg!(
        "User {} withdrew {} USDC from run #{} ({}/{})",
        ctx.accounts.user.key(),
        user_share,
        run_id,
        run.withdrawn_count,
        run.participant_count
    );

    Ok(())
}
```

---

## Fix #3: Enhanced Validation

### Add validation constants

```rust
// Add to top of lib.rs
const MIN_PARTICIPANTS: u16 = 2;
const MAX_PARTICIPANTS_LIMIT: u16 = 1000;
const MAX_VOTES: u8 = 12;
const MIN_RUN_DURATION: i64 = 3600; // 1 hour
const MAX_PLATFORM_FEE_BPS: u16 = 2000; // 20% max
```

### Update `create_run` validation

```rust
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
    require!(
        max_participants >= MIN_PARTICIPANTS && max_participants <= MAX_PARTICIPANTS_LIMIT,
        ErrorCode::InvalidParticipantLimit
    );

    // ... rest of function
}
```

### Update `start_run` validation

```rust
pub fn start_run(
    ctx: Context<ManageRun>,
    run_id: u64,
) -> Result<()> {
    let run = &mut ctx.accounts.run;
    
    require!(run.status == RunStatus::Waiting, ErrorCode::InvalidRunStatus);
    require!(
        run.participant_count >= MIN_PARTICIPANTS,
        ErrorCode::InsufficientParticipants
    );

    run.status = RunStatus::Active;
    run.started_at = Clock::get()?.unix_timestamp;

    msg!(
        "Run #{} started with {} participants and {} USDC",
        run_id,
        run.participant_count,
        run.total_deposited
    );
    Ok(())
}
```

### Update `settle_run` validation

```rust
pub fn settle_run(
    ctx: Context<SettleRun>,
    run_id: u64,
    final_balance: u64,
    participant_shares: Vec<ParticipantShare>,
) -> Result<()> {
    let run = &mut ctx.accounts.run;
    
    require!(run.status == RunStatus::Active, ErrorCode::InvalidRunStatus);
    
    // Validate minimum duration
    let current_time = Clock::get()?.unix_timestamp;
    require!(
        current_time >= run.started_at + MIN_RUN_DURATION,
        ErrorCode::RunTooShort
    );
    
    require!(
        participant_shares.len() == run.participant_count as usize,
        ErrorCode::InvalidSharesCount
    );

    // Verify vault balance
    let vault_balance = ctx.accounts.run_vault.amount;
    require!(vault_balance == final_balance, ErrorCode::VaultBalanceMismatch);

    // ... rest of settlement logic
}
```

### Update `update_vote_stats` validation

```rust
pub fn update_vote_stats(
    ctx: Context<UpdateVoteStats>,
    run_id: u64,
    user_pubkey: Pubkey,
    correct_votes: u8,
    total_votes: u8,
) -> Result<()> {
    let participation = &mut ctx.accounts.user_participation;
    
    require!(ctx.accounts.run.status == RunStatus::Active, ErrorCode::InvalidRunStatus);
    
    // Validate vote counts
    require!(total_votes <= MAX_VOTES, ErrorCode::InvalidVoteCount);
    require!(correct_votes <= total_votes, ErrorCode::InvalidVoteCount);
    
    // Ensure votes can only increase (prevent manipulation)
    require!(
        total_votes >= participation.total_votes,
        ErrorCode::VoteCountDecreased
    );
    
    participation.correct_votes = correct_votes;
    participation.total_votes = total_votes;

    Ok(())
}
```

---

## Fix #4: Add Missing Error Codes

```rust
#[error_code]
pub enum ErrorCode {
    // ... existing errors ...
    
    #[msg("Arithmetic overflow occurred")]
    ArithmeticOverflow,
    
    #[msg("Invalid vote count (must be 0-12)")]
    InvalidVoteCount,
    
    #[msg("Vote count cannot decrease")]
    VoteCountDecreased,
    
    #[msg("Run duration too short (minimum 1 hour)")]
    RunTooShort,
    
    #[msg("Insufficient participants (minimum 2 required)")]
    InsufficientParticipants,
}
```

---

## Testing Strategy

### Unit Tests for Fix #1 (Platform Fee)

```javascript
it("Collects platform fee on profitable run", async () => {
    // Setup: Run with 100 USDC deposits, 120 USDC final (20 USDC profit)
    // Platform fee: 15% of profit = 3 USDC
    
    const tx = await program.methods
        .settleRun(RUN_ID, finalBalance, shares)
        .accounts({ /* ... */ })
        .signers([platformAuthority])
        .rpc();
    
    const run = await program.account.run.fetch(runPda);
    assert.equal(run.platformFeeAmount.toNumber(), 3_000_000); // 3 USDC
    assert.equal(run.finalBalance.toNumber(), 117_000_000); // 120 - 3
    
    const feeVault = await getAccount(provider.connection, platformFeeVaultPda);
    assert.equal(feeVault.amount.toString(), "3000000");
});

it("Collects no fee on losing run", async () => {
    // Setup: Run with 100 USDC deposits, 80 USDC final (20 USDC loss)
    
    const tx = await program.methods
        .settleRun(RUN_ID, finalBalance, shares)
        .accounts({ /* ... */ })
        .rpc();
    
    const run = await program.account.run.fetch(runPda);
    assert.equal(run.platformFeeAmount.toNumber(), 0); // No fee on loss
    assert.equal(run.finalBalance.toNumber(), 80_000_000);
});
```

### Unit Tests for Fix #2 (Rounding)

```javascript
it("Last user gets all remaining vault balance", async () => {
    // Setup: 3 users with 10 USDC each, final balance 31 USDC
    // User 1 withdraws: 10 USDC
    // User 2 withdraws: 10 USDC
    // User 3 should get: 11 USDC (all remaining)
    
    await program.methods.withdraw(RUN_ID)
        .accounts({ user: user1 })
        .signers([user1])
        .rpc();
    
    await program.methods.withdraw(RUN_ID)
        .accounts({ user: user2 })
        .signers([user2])
        .rpc();
    
    const vaultBefore = await getAccount(provider.connection, runVaultPda);
    
    await program.methods.withdraw(RUN_ID)
        .accounts({ user: user3 })
        .signers([user3])
        .rpc();
    
    const vaultAfter = await getAccount(provider.connection, runVaultPda);
    assert.equal(vaultAfter.amount.toString(), "0"); // Empty vault
    
    const user3Balance = await getAccount(provider.connection, user3TokenAccount);
    assert.equal(user3Balance.amount, vaultBefore.amount); // Got all remaining
});
```

### Integration Test

```javascript
it("Complete run lifecycle with fees and bonuses", async () => {
    // 1. Create run
    // 2. Multiple users deposit
    // 3. Start run
    // 4. Update vote stats
    // 5. Settle with profit
    // 6. Verify platform fee collected
    // 7. All users withdraw
    // 8. Verify vault empty
    // 9. Verify total distributed = final_balance - platform_fee
});
```

---

## Migration Strategy

If you need to migrate existing on-chain data:

1. **Deploy new version** with updated account structures
2. **Disable old program** (pause)
3. **Data migration script**:
   - Read all old accounts
   - Create new accounts with additional fields
   - Transfer vault ownership
4. **Verify migration** with test withdrawals
5. **Enable new program**

---

## Deployment Checklist

- [ ] All critical fixes implemented
- [ ] Unit tests passing (100% coverage)
- [ ] Integration tests passing
- [ ] Fuzz tests for arithmetic
- [ ] Deploy to devnet
- [ ] Test with real users on devnet
- [ ] Professional security audit
- [ ] Bug bounty program
- [ ] Gradual mainnet rollout with caps
- [ ] Monitoring and alerts active


