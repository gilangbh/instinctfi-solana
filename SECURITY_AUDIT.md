# Instinct Trading Solana Program - Security Audit Report

**Program**: instinct_trading  
**Version**: 0.1.0  
**Auditor**: AI Security Analysis  
**Date**: October 28, 2025  
**Anchor Version**: 0.31.1  

---

## 🔴 CRITICAL ISSUES

### C-1: Platform Fee Not Implemented
**Severity**: Critical  
**Location**: `settle_run` instruction (lines 136-175)

**Description**:  
The platform defines `platform_fee_bps` during initialization but never deducts it during settlement. This means the platform generates no revenue.

**Current Code**:
```rust
pub fn settle_run(
    ctx: Context<SettleRun>,
    run_id: u64,
    final_balance: u64,
    participant_shares: Vec<ParticipantShare>,
) -> Result<()> {
    // ... validation ...
    run.final_balance = final_balance;
    // ❌ No platform fee deduction!
}
```

**Impact**:  
- Platform operates at zero revenue
- Economic model broken
- Cannot fund development/operations

**Recommendation**:
```rust
// Calculate platform fee
let platform_fee = (final_balance as u128)
    .checked_mul(ctx.accounts.platform.platform_fee_bps as u128)
    .unwrap()
    .checked_div(10000)
    .unwrap() as u64;

let profit = if final_balance > run.total_deposited {
    final_balance - run.total_deposited
} else {
    0
};

let fee_on_profit = (profit as u128)
    .checked_mul(ctx.accounts.platform.platform_fee_bps as u128)
    .unwrap()
    .checked_div(10000)
    .unwrap() as u64;

// Store fee for platform withdrawal
run.platform_fee_collected = fee_on_profit;
run.final_balance = final_balance - fee_on_profit;
```

---

### C-2: Participant Shares Parameter Unused
**Severity**: Critical  
**Location**: `settle_run` instruction (line 141)

**Description**:  
The function accepts `participant_shares: Vec<ParticipantShare>` but doesn't store or use it. Withdrawal calculation happens dynamically in `withdraw()`, which:
1. Allows attackers to manipulate shares if vote stats are incorrect
2. Makes calculations inconsistent across withdrawals
3. Creates race conditions if vault balance changes

**Current Code**:
```rust
pub fn settle_run(
    ctx: Context<SettleRun>,
    run_id: u64,
    final_balance: u64,
    participant_shares: Vec<ParticipantShare>, // ❌ Accepted but unused!
) -> Result<()> {
    // ... validation ...
    // Note: In production, you'd want to store this data in separate accounts
    // For MVP, we'll handle distribution through the withdraw instruction
}
```

**Impact**:  
- Share calculations inconsistent across withdrawals
- Last withdrawers may receive incorrect amounts if vault depletes
- No on-chain proof of agreed distribution
- Backend can't verify settlement accuracy

**Recommendation**:  
Either:
1. **Remove the parameter** and calculate dynamically (current behavior)
2. **Store shares on-chain** in UserParticipation accounts during settlement
3. **Implement merkle proof** system for scalability

**Preferred Solution**:
```rust
// Update UserParticipation during settlement
for share in participant_shares.iter() {
    // Load and update each participation account
    // This requires passing remaining accounts
}
```

---

### C-3: Withdrawal Share Calculation Has Rounding Issues
**Severity**: Critical  
**Location**: `withdraw` instruction (lines 188-199)

**Description**:  
The share calculation uses integer division which can lead to:
1. **Dust remaining in vault** that no one can withdraw
2. **Unfair distribution** where last withdrawer gets more/less
3. **Potential vault depletion** before all users withdraw

**Current Code**:
```rust
let base_share_numerator = (participation.deposit_amount as u128)
    .checked_mul(run.final_balance as u128)
    .unwrap();
let mut user_share = (base_share_numerator / run.total_deposited as u128) as u64;

// ❌ Problem: Sum of all shares may not equal final_balance due to rounding
```

**Example Exploit**:
```
3 users deposit 10 USDC each = 30 USDC total
Final balance = 31 USDC (1 USDC profit)

User 1 share: (10 * 31) / 30 = 10.333... = 10 USDC
User 2 share: (10 * 31) / 30 = 10.333... = 10 USDC  
User 3 share: (10 * 31) / 30 = 10.333... = 10 USDC

Total withdrawn: 30 USDC
Remaining in vault: 1 USDC (locked forever)
```

**Impact**:  
- Funds locked in vault permanently
- Unfair distribution
- Last user may fail to withdraw if vault empty

**Recommendation**:
```rust
// Track total withdrawn in Run account
run.total_withdrawn += user_share;

// Last user gets remaining vault balance
if run.participant_count - run.withdrawn_count == 1 {
    user_share = ctx.accounts.run_vault.amount;
} else {
    // Calculate proportional share
    let base_share_numerator = (participation.deposit_amount as u128)
        .checked_mul(run.final_balance as u128)
        .unwrap();
    user_share = (base_share_numerator / run.total_deposited as u128) as u64;
    
    // Add bonus...
}

run.withdrawn_count += 1;
```

---

## 🟠 HIGH SEVERITY ISSUES

### H-1: No Access Control on `update_vote_stats`
**Severity**: High  
**Location**: `update_vote_stats` instruction (lines 231-246)

**Description**:  
While the function requires `authority` to sign, anyone who knows the platform authority pubkey can attempt to call this. There's no verification that the caller is actually the backend service.

**Current Code**:
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
    
    participation.correct_votes = correct_votes;
    participation.total_votes = total_votes; // ❌ No verification!

    Ok(())
}
```

**Impact**:  
- If authority key compromised, attacker can manipulate vote stats
- Can give themselves 12/12 correct votes for max bonus
- Economic exploit: steal ~12% more from pool

**Recommendation**:
```rust
// Add validation
require!(total_votes <= 12, ErrorCode::InvalidVoteCount);
require!(correct_votes <= total_votes, ErrorCode::InvalidVoteCount);

// Consider using a separate "backend authority" key for this operation
// to limit blast radius if compromised
```

---

### H-2: Missing Validation in `settle_run`
**Severity**: High  
**Location**: `settle_run` instruction (lines 136-175)

**Description**:  
Several critical validations are missing:

1. **No check that all users have been updated with vote stats**
2. **No verification that participant_shares sum equals final_balance**
3. **No validation that run has been active for minimum duration**
4. **Vault balance check happens BEFORE settlement, not protecting against race conditions**

**Current Code**:
```rust
pub fn settle_run(
    ctx: Context<SettleRun>,
    run_id: u64,
    final_balance: u64,
    participant_shares: Vec<ParticipantShare>,
) -> Result<()> {
    let run = &mut ctx.accounts.run;
    
    require!(run.status == RunStatus::Active, ErrorCode::InvalidRunStatus);
    require!(participant_shares.len() == run.participant_count as usize, ErrorCode::InvalidSharesCount);

    // ❌ No validation that shares sum correctly
    // ❌ No minimum duration check
    // ❌ No verification all users updated
    
    let vault_balance = ctx.accounts.run_vault.amount;
    require!(vault_balance == final_balance, ErrorCode::VaultBalanceMismatch);
}
```

**Impact**:  
- Backend can settle immediately after starting (no trading time)
- Incorrect share distribution possible
- Race condition: vault balance could change between check and settlement

**Recommendation**:
```rust
// Validate minimum duration
let current_time = Clock::get()?.unix_timestamp;
let min_duration = 3600; // 1 hour minimum
require!(
    current_time >= run.started_at + min_duration,
    ErrorCode::RunTooShort
);

// Validate shares sum (if keeping the parameter)
let total_shares: u64 = participant_shares.iter()
    .map(|s| s.share_amount)
    .sum();
require!(
    total_shares == final_balance,
    ErrorCode::SharesSumMismatch
);

// Or better: remove parameter entirely and calculate on-demand
```

---

### H-3: Bonus Calculation Can Exceed Vault Balance
**Severity**: High  
**Location**: `withdraw` instruction (lines 196-199)

**Description**:  
The bonus is added to base share WITHOUT checking if total withdrawals would exceed vault balance. With 12% max bonus per user, multiple users with perfect votes could drain vault before everyone withdraws.

**Current Code**:
```rust
let correct_vote_bonus_bps = participation.correct_votes as u64 * 100; // 1% per correct vote
let bonus = (user_share as u128 * correct_vote_bonus_bps as u128 / 10000) as u64;
user_share += bonus;

// ❌ Only checks AFTER calculation
require!(user_share <= ctx.accounts.run_vault.amount, ErrorCode::InsufficientVaultFunds);
```

**Example Exploit**:
```
10 users each deposit 10 USDC = 100 USDC
All users get 12/12 correct votes
Final balance = 100 USDC (break even)

Each user base share: 10 USDC
Each user bonus: 10 * 0.12 = 1.2 USDC
Each user total: 11.2 USDC

Total owed: 10 * 11.2 = 112 USDC
Vault has: 100 USDC
Shortfall: 12 USDC (last users can't withdraw!)
```

**Impact**:  
- Vault insolvency
- Last users unable to withdraw
- Race condition to withdraw first

**Recommendation**:
```rust
// Option 1: Cap total bonuses at available profit
let profit = if run.final_balance > run.total_deposited {
    run.final_balance - run.total_deposited
} else {
    0
};

// Bonus comes from profit only, not principal
let max_bonus_pool = profit; // or profit * 0.5 (50% of profit for bonuses)

// Option 2: Make bonuses relative to profit, not deposit
let base_share = (participation.deposit_amount as u128 * run.final_balance as u128 
    / run.total_deposited as u128) as u64;

if run.final_balance > run.total_deposited {
    let profit_share = base_share - participation.deposit_amount;
    let bonus = (profit_share * correct_vote_bonus_bps / 10000) as u64;
    user_share = base_share + bonus;
} else {
    user_share = base_share; // No bonus on losses
}
```

---

### H-4: No Protection Against Deposit Front-Running
**Severity**: High  
**Location**: `deposit` instruction (lines 74-116)

**Description**:  
If voting results are publicly visible before run starts, sophisticated users could:
1. Monitor for profitable voting patterns
2. Deposit at the last moment before `start_run`
3. Get proportional share without contributing to voting/strategy

**Current Code**:
```rust
pub fn deposit(
    ctx: Context<Deposit>,
    run_id: u64,
    amount: u64,
) -> Result<()> {
    require!(run.status == RunStatus::Waiting, ErrorCode::RunNotInWaitingPhase);
    // ❌ No deposit deadline before run starts
    // ❌ No minimum waiting period
}
```

**Impact**:  
- Users exploit voting information asymmetry
- Undermines gamification incentives
- Early voters subsidize late joiners

**Recommendation**:
```rust
// Add deposit deadline in Run account
pub struct Run {
    // ... existing fields ...
    pub deposit_deadline: i64, // Unix timestamp
}

// Validate in deposit
require!(
    Clock::get()?.unix_timestamp <= run.deposit_deadline,
    ErrorCode::DepositsClosedForRun
);
```

---

## 🟡 MEDIUM SEVERITY ISSUES

### M-1: Missing Minimum Participant Check
**Severity**: Medium  
**Location**: `start_run` instruction (lines 118-134)

**Description**:  
While `start_run` checks `participant_count > 0`, it doesn't enforce a meaningful minimum. A single user could start a run with themselves, guaranteeing 100% return (minus platform fee).

**Current Code**:
```rust
pub fn start_run(
    ctx: Context<ManageRun>,
    run_id: u64,
) -> Result<()> {
    require!(run.participant_count > 0, ErrorCode::NoParticipants);
    // ❌ Should require minimum 2+ participants
}
```

**Recommendation**:
```rust
const MIN_PARTICIPANTS: u16 = 5; // Or make configurable per run

require!(
    run.participant_count >= MIN_PARTICIPANTS,
    ErrorCode::InsufficientParticipants
);
```

---

### M-2: No Maximum Deposit Total Cap
**Severity**: Medium  
**Location**: `deposit` instruction

**Description**:  
While individual deposits have min/max limits, there's no cap on total run size. This could lead to:
1. Runs too large for backend to execute trades efficiently
2. Liquidity issues on Drift Protocol
3. Slippage exceeding acceptable thresholds

**Recommendation**:
```rust
pub struct Run {
    // ... existing fields ...
    pub max_total_deposit: u64,
}

// In deposit function
require!(
    run.total_deposited + amount <= run.max_total_deposit,
    ErrorCode::RunCapExceeded
);
```

---

### M-3: Emergency Withdraw Lacks Multi-Sig
**Severity**: Medium  
**Location**: `emergency_withdraw` instruction (lines 263-290)

**Description**:  
A single compromised authority key can drain all funds during pause. No time-lock, no multi-sig.

**Current Code**:
```rust
pub fn emergency_withdraw(
    ctx: Context<EmergencyWithdraw>,
    run_id: u64,
    amount: u64,
) -> Result<()> {
    require!(ctx.accounts.platform.is_paused, ErrorCode::PlatformNotPaused);
    // ❌ Single signature, immediate execution
}
```

**Recommendation**:
```rust
// Implement using Squads Protocol or similar multi-sig
// Alternatively, add timelock:

pub struct Platform {
    // ... existing fields ...
    pub emergency_withdraw_delay: i64, // 24-48 hours
    pub pending_withdrawal: Option<PendingWithdrawal>,
}

pub struct PendingWithdrawal {
    pub run_id: u64,
    pub amount: u64,
    pub initiated_at: i64,
    pub destination: Pubkey,
}

// Two-step process:
// 1. initiate_emergency_withdraw (stores pending)
// 2. execute_emergency_withdraw (after delay)
```

---

### M-4: Vote Stats Can Be Updated Multiple Times
**Severity**: Medium  
**Location**: `update_vote_stats` instruction (lines 231-246)

**Description**:  
No check prevents backend from calling this multiple times per run, potentially overwriting vote statistics.

**Current Code**:
```rust
pub fn update_vote_stats(
    ctx: Context<UpdateVoteStats>,
    run_id: u64,
    user_pubkey: Pubkey,
    correct_votes: u8,
    total_votes: u8,
) -> Result<()> {
    participation.correct_votes = correct_votes;
    participation.total_votes = total_votes; // ❌ Overwrites previous values
}
```

**Recommendation**:
```rust
// Make it append-only
require!(
    total_votes > participation.total_votes,
    ErrorCode::VoteStatsAlreadyFinal
);

// Or add incremental updates
pub fn increment_vote_stats(
    ctx: Context<UpdateVoteStats>,
    was_correct: bool,
) -> Result<()> {
    participation.total_votes += 1;
    if was_correct {
        participation.correct_votes += 1;
    }
    Ok(())
}
```

---

### M-5: No Cancellation Mechanism for Runs
**Severity**: Medium  
**Location**: Missing functionality

**Description**:  
If a run fails to reach minimum participants or encounters issues, there's no way to cancel and refund deposits.

**Recommendation**:
```rust
pub fn cancel_run(
    ctx: Context<ManageRun>,
    run_id: u64,
) -> Result<()> {
    let run = &mut ctx.accounts.run;
    
    require!(
        run.status == RunStatus::Waiting,
        ErrorCode::CannotCancelActiveRun
    );
    
    // Allow cancellation if:
    // - Not enough participants
    // - Past deadline without starting
    // - Emergency situation
    
    run.status = RunStatus::Cancelled;
    Ok(())
}

// Users can then withdraw their original deposits
```

---

## 🔵 LOW SEVERITY ISSUES

### L-1: Inefficient Space Allocation
**Severity**: Low  
**Location**: Account size calculations

**Description**:  
Using manual space calculation instead of Anchor's automatic sizing.

**Current Code**:
```rust
impl Platform {
    pub const LEN: usize = 8 + 32 + 2 + 8 + 1 + 1;
}
```

**Recommendation**:
```rust
// Let Anchor calculate automatically
#[account]
#[derive(InitSpace)]
pub struct Platform {
    pub authority: Pubkey,
    pub platform_fee_bps: u16,
    pub total_runs: u64,
    pub is_paused: bool,
    pub bump: u8,
}

// Then use in Context
#[account(
    init,
    payer = authority,
    space = 8 + Platform::INIT_SPACE, // Automatic
    seeds = [b"platform"],
    bump
)]
```

---

### L-2: Missing Events/Logging
**Severity**: Low  
**Location**: Throughout

**Description**:  
Only using `msg!()` for logging. No structured events for indexers.

**Recommendation**:
```rust
#[event]
pub struct RunCreated {
    pub run_id: u64,
    pub min_deposit: u64,
    pub max_deposit: u64,
    pub max_participants: u16,
    pub created_at: i64,
}

#[event]
pub struct UserDeposited {
    pub run_id: u64,
    pub user: Pubkey,
    pub amount: u64,
    pub participant_count: u16,
}

// Emit in functions
emit!(RunCreated {
    run_id,
    min_deposit,
    max_deposit,
    max_participants,
    created_at: Clock::get()?.unix_timestamp,
});
```

---

### L-3: No Validation on `max_participants` Upper Bound
**Severity**: Low  
**Location**: `create_run` instruction (line 48)

**Description**:  
While it checks `> 0`, there's no upper limit. Setting to 65535 could cause issues.

**Current Code**:
```rust
require!(max_participants > 0, ErrorCode::InvalidParticipantLimit);
```

**Recommendation**:
```rust
const MAX_PARTICIPANTS_LIMIT: u16 = 1000; // Reasonable limit

require!(
    max_participants > 0 && max_participants <= MAX_PARTICIPANTS_LIMIT,
    ErrorCode::InvalidParticipantLimit
);
```

---

### L-4: Timestamp Usage Instead of Slots
**Severity**: Low  
**Location**: Multiple locations using `Clock::get()?.unix_timestamp`

**Description**:  
Unix timestamps can be manipulated slightly by validators. Slots are more reliable.

**Recommendation**:
```rust
// Use Clock::get()?.slot for critical timing
let current_slot = Clock::get()?.slot;

// If human-readable time needed, store both
pub created_at_slot: u64,
pub created_at_timestamp: i64,
```

---

### L-5: No Platform Fee Update Mechanism
**Severity**: Low  
**Location**: Platform struct

**Description**:  
Platform fee is set during initialization and can never be changed.

**Recommendation**:
```rust
pub fn update_platform_fee(
    ctx: Context<AdminAction>,
    new_fee_bps: u16,
) -> Result<()> {
    require!(new_fee_bps <= 10000, ErrorCode::InvalidFee);
    ctx.accounts.platform.platform_fee_bps = new_fee_bps;
    
    emit!(PlatformFeeUpdated {
        old_fee: ctx.accounts.platform.platform_fee_bps,
        new_fee: new_fee_bps,
    });
    
    Ok(())
}
```

---

## ℹ️ INFORMATIONAL / BEST PRACTICES

### I-1: Consider Using Constants
```rust
const MIN_DEPOSIT_AMOUNT: u64 = 1_000_000; // 1 USDC minimum
const MAX_DEPOSIT_AMOUNT: u64 = 1_000_000_000; // 1000 USDC maximum
const MAX_PLATFORM_FEE_BPS: u16 = 2000; // 20% maximum fee
const VOTE_BONUS_BPS_PER_VOTE: u64 = 100; // 1% per vote
const MAX_VOTES: u8 = 12;
```

---

### I-2: Add Version Field for Upgradeability
```rust
pub struct Platform {
    pub version: u8, // For future migrations
    // ... rest of fields
}
```

---

### I-3: Implement Account Validation Helpers
```rust
#[account]
impl Run {
    pub fn is_accepting_deposits(&self) -> bool {
        self.status == RunStatus::Waiting && !self.is_full()
    }
    
    pub fn is_full(&self) -> bool {
        self.participant_count >= self.max_participants
    }
    
    pub fn has_minimum_participants(&self, min: u16) -> bool {
        self.participant_count >= min
    }
}
```

---

### I-4: Add Comprehensive Error Messages
Current errors are good but could be more descriptive:
```rust
#[error_code]
pub enum ErrorCode {
    #[msg("Invalid fee percentage (must be 0-10000 basis points, i.e., 0-100%)")]
    InvalidFee,
    
    #[msg("Platform is paused - deposits and new runs are disabled")]
    PlatformPaused,
    
    // ... etc
}
```

---

### I-5: Document Invariants
Add documentation about expected invariants:
```rust
/// Invariants:
/// - sum(all user deposits) == run.total_deposited
/// - sum(all user withdrawals) == run.final_balance (after all withdrawals)
/// - run.participant_count == number of UserParticipation accounts for this run
/// - vault.amount >= sum(remaining withdrawals)
#[account]
pub struct Run {
    // ...
}
```

---

## 📊 RISK SUMMARY

| Severity | Count | Description |
|----------|-------|-------------|
| 🔴 Critical | 3 | Platform fee not collected, share calculation issues, rounding errors |
| 🟠 High | 4 | Access control gaps, validation missing, bonus calculation flaws |
| 🟡 Medium | 5 | Minimum participants, emergency withdraw risks, cancellation missing |
| 🔵 Low | 5 | Space allocation, events, limits, timestamps |
| ℹ️ Info | 5 | Best practices and code quality improvements |

**Total Issues**: 22

---

## 🛠️ PRIORITY FIXES (Before Mainnet)

### Must Fix (Critical):
1. ✅ Implement platform fee deduction in `settle_run`
2. ✅ Fix withdrawal share calculation rounding
3. ✅ Resolve participant shares storage/usage

### Should Fix (High):
4. ✅ Add vote stats validation
5. ✅ Enhance `settle_run` validation
6. ✅ Fix bonus calculation to prevent vault insolvency
7. ✅ Add deposit deadline mechanism

### Consider Fixing (Medium):
8. Add minimum participant requirement
9. Implement run cancellation
10. Add multi-sig or timelock for emergency withdraw

---

## 🔒 SECURITY RECOMMENDATIONS

### Immediate Actions:
1. **Conduct professional audit** by Kudelski, OtterSec, or Neodyme
2. **Bug bounty program** on Immunefi before mainnet
3. **Gradual rollout** with deposit caps on initial runs
4. **Monitor vault balances** and pause if anomalies detected

### Architecture Improvements:
1. **Separate platform authority from backend authority**
2. **Implement timelock for sensitive operations**
3. **Consider upgrading to multi-sig (Squads)**
4. **Add circuit breakers** for unusual activity

### Testing Requirements:
1. ✅ Fuzz testing for arithmetic operations
2. ✅ Invariant testing for vault balances
3. ✅ Integration testing with real Drift Protocol
4. ✅ Stress testing with max participants
5. ✅ Edge case testing (1 wei deposits, max values, etc.)

---

## 📝 CONCLUSION

The Instinct Trading program demonstrates **solid foundational architecture** with proper use of PDAs, state management, and Anchor patterns. However, **several critical issues must be addressed** before mainnet deployment:

**Strengths:**
✅ Good use of Anchor framework  
✅ Proper PDA derivation  
✅ Basic access control structure  
✅ State machine pattern for runs  
✅ Comprehensive test coverage  

**Critical Gaps:**
❌ Platform fee not implemented  
❌ Share calculation rounding issues  
❌ Bonus calculation can cause insolvency  
❌ Missing key validations  

**Recommendation**: **DO NOT DEPLOY TO MAINNET** until critical issues are resolved and a professional security audit is completed.

**Estimated Effort to Fix Critical Issues**: 3-5 days of development + testing

---

**Next Steps:**
1. Address all Critical and High severity issues
2. Engage professional auditing firm
3. Implement comprehensive integration tests
4. Consider formal verification for share calculations
5. Deploy to devnet with deposit caps
6. Run bug bounty program
7. Gradual mainnet rollout with monitoring


