# Attack Scenarios - Security Analysis

This document demonstrates concrete attack vectors and exploits in the current implementation.

---

## 🚨 Scenario 1: Bonus Vault Drain Attack

**Vulnerability**: Bonus calculation can exceed available vault balance  
**Severity**: Critical  
**Attacker Profit**: Up to 20% of vault balance

### Setup

```
Run Configuration:
- 10 participants
- Each deposits: 50 USDC
- Total deposited: 500 USDC
- Trading result: Break even (0% profit)
- Final balance: 500 USDC
```

### Attack Steps

1. **Attacker controls backend** (compromised authority key)
2. **Manipulates vote stats** for all users:
   ```rust
   update_vote_stats(user_1, correct_votes: 12, total_votes: 12)
   update_vote_stats(user_2, correct_votes: 12, total_votes: 12)
   // ... repeat for all 10 users
   ```

3. **Settles run** with break-even balance:
   ```rust
   settle_run(run_id: 1, final_balance: 500 USDC)
   ```

4. **Users begin withdrawing**:
   ```
   User 1: 50 + (50 * 12%) = 56 USDC ✅
   User 2: 50 + (50 * 12%) = 56 USDC ✅
   User 3: 50 + (50 * 12%) = 56 USDC ✅
   User 4: 50 + (50 * 12%) = 56 USDC ✅
   User 5: 50 + (50 * 12%) = 56 USDC ✅
   User 6: 50 + (50 * 12%) = 56 USDC ✅
   User 7: 50 + (50 * 12%) = 56 USDC ✅
   User 8: 50 + (50 * 12%) = 56 USDC ✅
   
   Total withdrawn: 448 USDC
   Vault remaining: 52 USDC
   
   User 9: Expects 56 USDC, gets 52 USDC ⚠️
   User 10: FAILS - insufficient vault funds ❌
   ```

### Impact

- **Users 9-10 cannot withdraw** their full share
- **52 USDC shortfall** from expected distribution
- **Race condition**: First withdrawers benefit at expense of last
- **Creates bank run scenario**: Users rush to withdraw first

### Fix

Bonus should only apply to profit portion:

```rust
if run.final_balance > run.total_deposited {
    let profit = run.final_balance - run.total_deposited;
    let user_profit_share = (deposit * profit) / total_deposited;
    let bonus = (user_profit_share * correct_votes) / 100;
    user_share = deposit + user_profit_share + bonus;
} else {
    user_share = (deposit * final_balance) / total_deposited;
    // No bonus on losses
}
```

---

## 🚨 Scenario 2: Rounding Dust Accumulation

**Vulnerability**: Integer division leaves dust in vault  
**Severity**: Critical  
**Platform Loss**: Accumulates over time

### Setup

```
Run #1:
- 3 participants
- User A deposits: 10 USDC
- User B deposits: 10 USDC  
- User C deposits: 10 USDC
- Total: 30 USDC
- Final balance: 31 USDC (1 USDC profit)
```

### Attack Steps

1. **Run settles** with 31 USDC final balance

2. **Share calculation** (integer division):
   ```rust
   User A: (10 * 31) / 30 = 310 / 30 = 10.333... = 10 USDC
   User B: (10 * 31) / 30 = 310 / 30 = 10.333... = 10 USDC
   User C: (10 * 31) / 30 = 310 / 30 = 10.333... = 10 USDC
   
   Total distributed: 30 USDC
   Remaining in vault: 1 USDC
   ```

3. **Dust accumulates**:
   ```
   Run #1 dust: 1 USDC
   Run #2 dust: 0.8 USDC
   Run #3 dust: 1.2 USDC
   Run #4 dust: 0.5 USDC
   ...
   After 100 runs: ~50 USDC locked permanently
   ```

### Impact

- **Funds locked forever** (no account can claim dust)
- **Platform loses money** over time
- **Accounting mismatch**: Vaults contain unclaimed funds
- **Scales with volume**: More runs = more dust

### Proof

```javascript
// Test demonstrating dust accumulation
it("leaves dust in vault after all withdrawals", async () => {
  const finalBalance = new anchor.BN(31_000_000); // 31 USDC
  
  await program.methods.settleRun(RUN_ID, finalBalance, shares).rpc();
  
  // All users withdraw
  await program.methods.withdraw(RUN_ID).accounts({ user: user1 }).rpc();
  await program.methods.withdraw(RUN_ID).accounts({ user: user2 }).rpc();
  await program.methods.withdraw(RUN_ID).accounts({ user: user3 }).rpc();
  
  const vault = await getAccount(provider.connection, vaultPda);
  assert.ok(vault.amount > 0); // ❌ Dust remaining!
  console.log("Dust left in vault:", vault.amount); // ~1 USDC
});
```

### Fix

Last user gets all remaining balance:

```rust
if is_last_withdrawal {
    user_share = vault.amount; // Give all remaining
} else {
    user_share = calculate_proportional_share();
}
```

---

## 🚨 Scenario 3: Platform Fee Circumvention

**Vulnerability**: Platform fee configured but never collected  
**Severity**: Critical  
**Platform Loss**: 100% of intended revenue

### Setup

```
Platform Configuration:
- Platform fee: 15%
- Expected to take 15% of profits

Run:
- Total deposited: 1000 USDC
- Final balance: 1500 USDC
- Profit: 500 USDC
- Expected platform fee: 75 USDC (15% of 500)
```

### Attack Steps

1. **Run completes successfully**

2. **Settlement happens**:
   ```rust
   settle_run(run_id: 1, final_balance: 1500 USDC)
   
   // Current code:
   run.final_balance = 1500 USDC
   // ❌ No fee deducted!
   ```

3. **Users withdraw full amount**:
   ```
   Total available for withdrawal: 1500 USDC
   Platform fee collected: 0 USDC ❌
   Expected fee: 75 USDC
   Platform loss: 75 USDC (100% of intended revenue)
   ```

### Impact

- **Zero platform revenue** despite configuration
- **Cannot fund operations** or development
- **Unsustainable business model**
- **Investor value at risk**

### Proof

```javascript
it("platform fee not collected", async () => {
  const initialDeposit = 1000_000_000;
  const finalBalance = 1500_000_000; // 50% profit
  const expectedFee = 75_000_000; // 15% of 500 USDC profit
  
  await program.methods.settleRun(RUN_ID, finalBalance, shares).rpc();
  
  const run = await program.account.run.fetch(runPda);
  const platform = await program.account.platform.fetch(platformPda);
  
  // Check platform fee collected
  assert.equal(platform.totalFeesCollected.toNumber(), 0); // ❌ ZERO!
  
  // Final balance should be: 1500 - 75 = 1425 USDC
  assert.equal(run.finalBalance.toNumber(), 1500_000_000); // ❌ No deduction!
});
```

---

## 🚨 Scenario 4: Vote Manipulation Attack

**Vulnerability**: No validation on vote statistics updates  
**Severity**: High  
**Attacker Profit**: Up to 12% extra from pool

### Setup

```
Attacker Profile:
- Regular user participating in run
- Compromised backend authority key
- Can call update_vote_stats

Run:
- 10 participants each deposit 100 USDC
- Total: 1000 USDC
- Final balance: 1100 USDC (100 USDC profit)
```

### Attack Steps

1. **Attacker deposits** normally:
   ```rust
   deposit(run_id: 1, amount: 100 USDC)
   ```

2. **Run progresses**, attacker votes poorly:
   ```
   Actual voting record:
   - Round 1: Wrong
   - Round 2: Wrong  
   - Round 3: Right
   - ... (continues)
   - Actual score: 3/12 correct
   ```

3. **Attacker uses compromised key** to manipulate their stats:
   ```rust
   update_vote_stats(
     run_id: 1,
     user: attacker_pubkey,
     correct_votes: 12,  // ❌ LIE! Actually 3
     total_votes: 12
   )
   ```

4. **Settlement and withdrawal**:
   ```
   Normal user share: 100 + (10 * 0%) = 110 USDC
   Attacker share: 100 + (10 * 12%) = 111.2 USDC
   
   Attacker gain: 1.2 USDC
   Other users loss: 1.2 USDC distributed among them
   ```

### Impact

- **Attacker steals from honest participants**
- **Undermines voting mechanism integrity**
- **Requires compromised authority key** (realistic if backend hacked)
- **Can be repeated** across multiple runs

### Proof

```javascript
it("allows vote stat manipulation", async () => {
  // Attacker has 3 correct votes in reality
  const actualCorrect = 3;
  
  // But calls update with fake stats
  await program.methods
    .updateVoteStats(RUN_ID, attackerPubkey, 12, 12) // ❌ Lie!
    .accounts({ authority: platformAuthority })
    .signers([platformAuthority])
    .rpc();
  
  // Later during withdrawal
  await program.methods.withdraw(RUN_ID)
    .accounts({ user: attacker })
    .signers([attacker])
    .rpc();
  
  const participation = await program.account.userParticipation.fetch(
    participationPda
  );
  
  // Attacker got bonus for 12/12 when they only earned 3/12
  const expectedShare = 110_000_000; // Fair share
  const actualShare = participation.finalShare.toNumber();
  assert.ok(actualShare > expectedShare); // ❌ Attacker profited!
});
```

### Fix

Add validation and prevent manipulation:

```rust
require!(total_votes <= 12, ErrorCode::InvalidVoteCount);
require!(correct_votes <= total_votes, ErrorCode::InvalidVoteCount);

// Prevent overwriting (append-only)
require!(
    total_votes >= participation.total_votes,
    ErrorCode::VoteCountDecreased
);

// Consider: Store individual vote results on-chain for auditability
```

---

## 🚨 Scenario 5: Front-Running Deposit Attack

**Vulnerability**: No deposit deadline before run starts  
**Severity**: Medium-High  
**Attacker Profit**: Risk-free profit from information asymmetry

### Setup

```
Public Information:
- Run is in "Waiting" state
- 50 users already deposited
- Voting trends publicly visible
- Current voting shows 80% accuracy (profitable pattern)

Run Configuration:
- Max participants: 100
- 50 spots remaining
```

### Attack Steps

1. **Sophisticated user monitors** voting API/frontend:
   ```javascript
   // Off-chain monitoring
   setInterval(async () => {
     const votes = await fetchVotingStats(runId);
     const accuracy = votes.correct / votes.total;
     
     if (accuracy > 0.75 && !hasDeposited) {
       // Profitable pattern detected!
       await depositLastMinute();
     }
   }, 1000);
   ```

2. **User waits** until voting shows strong profit signal

3. **Deposits at last moment** before `start_run`:
   ```rust
   // 1 second before backend calls start_run
   deposit(run_id: 1, amount: 100 USDC)
   ```

4. **Attacker gets full share** despite minimal contribution:
   ```
   Early users: Voted 12 times, shaped strategy
   Attacker: Voted 0 times, free-rode on others' work
   
   Both get equal base share of profits!
   ```

### Impact

- **Undermines gamification** incentives
- **Early voters subsidize late joiners**
- **Information asymmetry exploitation**
- **Sophisticated users always win**
- **Casual users always lose**

### Fix

Add deposit deadline:

```rust
pub struct Run {
    pub deposit_deadline: i64, // e.g., 1 hour before start
}

// In deposit function
require!(
    Clock::get()?.unix_timestamp <= run.deposit_deadline,
    ErrorCode::DepositsClosedForRun
);
```

---

## 🚨 Scenario 6: Single User Run Attack

**Vulnerability**: No minimum participant requirement  
**Severity**: Medium  
**Attacker Profit**: Guaranteed break-even (minus fees)

### Setup

```
Attacker Plan:
- Create run with 1 participant (themselves)
- Control 100% of voting
- Execute conservative strategy
```

### Attack Steps

1. **Run created**:
   ```rust
   create_run(
     run_id: 1,
     min_deposit: 10 USDC,
     max_deposit: 100 USDC,
     max_participants: 100
   )
   ```

2. **Only attacker deposits**:
   ```rust
   deposit(run_id: 1, amount: 100 USDC)
   ```

3. **Backend starts run** (only checks > 0 participants):
   ```rust
   start_run(run_id: 1) // ✅ Passes validation
   ```

4. **Attacker trades conservatively**:
   ```
   Total capital: 100 USDC
   Strategy: Maximum hedging, minimal risk
   Result: 101 USDC (1% gain, almost guaranteed)
   
   Platform fee: 15% of 1 USDC = 0.15 USDC
   Attacker profit: 0.85 USDC
   ```

5. **Attacker withdraws**:
   ```
   Deposit: 100 USDC
   Withdrawal: 100.85 USDC
   Net gain: 0.85 USDC (with minimal risk)
   ```

### Impact

- **Gaming the system** for risk-free returns
- **Defeats purpose** of collective trading
- **Platform provides** solo trading infrastructure
- **Not scalable** - but proves vulnerability

### Fix

```rust
const MIN_PARTICIPANTS: u16 = 5;

pub fn start_run(ctx: Context<ManageRun>, run_id: u64) -> Result<()> {
    require!(
        run.participant_count >= MIN_PARTICIPANTS,
        ErrorCode::InsufficientParticipants
    );
    // ...
}
```

---

## 🚨 Scenario 7: Emergency Withdraw Rug Pull

**Vulnerability**: Single-sig emergency withdraw with no timelock  
**Severity**: Critical  
**Potential Loss**: 100% of all vault funds

### Setup

```
Platform State:
- 10 active runs
- Total value locked: 50,000 USDC
- Single authority key controls emergency functions

Attack Vector:
- Compromised authority key (hacked backend, insider threat, etc.)
```

### Attack Steps

1. **Attacker gains access** to authority keypair:
   ```
   Possible vectors:
   - Backend server compromise
   - Insider threat
   - Key management vulnerability
   - Social engineering
   ```

2. **Attacker pauses platform**:
   ```rust
   pause_platform() // ✅ Only requires authority signature
   ```

3. **Immediately drains all vaults**:
   ```rust
   for each run_vault {
     emergency_withdraw(
       run_id: run.id,
       amount: vault.full_balance,
       destination: attacker_wallet
     )
   }
   ```

4. **Total time**: < 30 seconds from pause to drain

5. **Users return** to find:
   ```
   All vaults: 0 USDC
   Users can't withdraw: "Insufficient vault funds"
   Funds gone: Moved to attacker wallet
   ```

### Impact

- **Total loss of user funds**
- **No recovery possible**
- **Instantaneous execution** (no time for intervention)
- **Single point of failure**
- **Platform reputation destroyed**

### Fix

Implement multi-sig and timelock:

```rust
pub struct Platform {
    pub emergency_withdraw_delay: i64, // 48 hours
    pub pending_withdrawal: Option<PendingWithdrawal>,
}

pub fn initiate_emergency_withdraw(
    ctx: Context<EmergencyWithdraw>,
    run_id: u64,
    amount: u64,
) -> Result<()> {
    // Step 1: Request withdrawal
    ctx.accounts.platform.pending_withdrawal = Some(PendingWithdrawal {
        run_id,
        amount,
        initiated_at: Clock::get()?.unix_timestamp,
    });
    
    // Emits event - community can react!
    Ok(())
}

pub fn execute_emergency_withdraw(
    ctx: Context<EmergencyWithdraw>,
) -> Result<()> {
    // Step 2: Execute after delay
    let pending = ctx.accounts.platform.pending_withdrawal.unwrap();
    let elapsed = Clock::get()?.unix_timestamp - pending.initiated_at;
    
    require!(
        elapsed >= ctx.accounts.platform.emergency_withdraw_delay,
        ErrorCode::TimelockNotExpired
    );
    
    // Transfer funds
    // ...
}
```

**Better**: Use Squads Protocol multi-sig (3-of-5 signers)

---

## Summary of Attack Vectors

| Attack | Severity | Ease | Profit | Fix Difficulty |
|--------|----------|------|--------|----------------|
| Bonus Vault Drain | Critical | Easy | 20% | Medium |
| Rounding Dust | Critical | Passive | Accumulates | Easy |
| Fee Circumvention | Critical | N/A | 100% revenue loss | Easy |
| Vote Manipulation | High | Medium | 12% | Easy |
| Front-Running | Medium | Hard | Variable | Medium |
| Single User Run | Medium | Easy | Low | Easy |
| Emergency Rug Pull | Critical | Easy | 100% TVL | Hard |

**Total Estimated Risk**: **CRITICAL** - Do not deploy to mainnet

---

## Recommended Testing

### Fuzz Testing

```rust
// Use cargo-fuzz or similar
fuzz_target!(|data: FuzzData| {
    // Test withdrawal with random values
    let deposits: Vec<u64> = data.deposits;
    let final_balance: u64 = data.final_balance;
    
    // Ensure: sum(withdrawals) <= final_balance
    // Ensure: no dust > acceptable_threshold
    // Ensure: bonus doesn't cause insolvency
});
```

### Invariant Testing

```rust
// After every operation, verify:
assert!(vault_balance >= sum_of_remaining_withdrawals);
assert!(run.total_deposited == sum_of_all_user_deposits);
assert!(run.participant_count == actual_participation_accounts);
```

### Exploit Reproduction Tests

Create tests for each attack scenario above to prove vulnerabilities and verify fixes.


