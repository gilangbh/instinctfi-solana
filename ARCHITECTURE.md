# Instinct Trading - Program Architecture

**Visual guide to the Solana program structure and data flow**

---

## 📐 System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         INSTINCT TRADING                         │
│                      Solana Smart Contract                       │
└─────────────────────────────────────────────────────────────────┘

┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│   Frontend   │◄───────►│   Backend    │◄───────►│    Solana    │
│   (React)    │         │  (Node.js)   │         │   Program    │
│              │         │              │         │   (Anchor)   │
└──────────────┘         └──────────────┘         └──────────────┘
     │                         │                         │
     │ User Actions           │ Admin Actions           │
     │ - Connect wallet       │ - Create runs           │ On-Chain:
     │ - Deposit USDC         │ - Start runs            │ - USDC custody
     │ - Withdraw             │ - Settle runs           │ - Fund distribution
     │                        │ - Update votes          │ - Access control
     │                        │                         │
     │                        │ Off-Chain:              │
     │                        │ - Voting logic          │
     │                        │ - Trade execution       │
     │                        │ - Drift Protocol        │
     └────────────────────────┴─────────────────────────┘
```

---

## 🏗️ Account Structure

### Program Accounts (PDAs)

```
instinct_trading (Program)
│
├─ Platform PDA
│  └─ Seeds: ["platform"]
│  └─ Contains: authority, fee_bps, total_runs, is_paused
│  └─ Authority: Platform admin (multi-sig recommended)
│
├─ Platform Fee Vault PDA
│  └─ Seeds: ["platform_fee_vault"]
│  └─ Contains: SPL Token Account (USDC)
│  └─ Authority: Platform PDA
│  └─ Purpose: Collect platform fees from profitable runs
│
├─ Run PDA (per run)
│  └─ Seeds: ["run", run_id (8 bytes)]
│  └─ Contains: status, deposits, participants, timestamps
│  └─ Authority: Platform authority
│  └─ Lifecycle: Waiting → Active → Settled
│
├─ Run Vault PDA (per run)
│  └─ Seeds: ["vault", run_id (8 bytes)]
│  └─ Contains: SPL Token Account (USDC)
│  └─ Authority: Run PDA
│  └─ Purpose: Hold user deposits during run
│
└─ User Participation PDA (per user per run)
   └─ Seeds: ["participation", run_id, user_pubkey]
   └─ Contains: deposit amount, vote stats, withdrawal status
   └─ Purpose: Track user's participation in specific run
```

### Account Sizes

```
Platform:         ~60 bytes  (8 discriminator + 52 data)
Run:              ~112 bytes (8 discriminator + 104 data)
UserParticipation: ~68 bytes (8 discriminator + 60 data)
Token Accounts:   ~165 bytes (SPL Token standard)
```

---

## 🔄 Run Lifecycle

### State Machine

```
           create_run()
               ↓
        ┌─────────────┐
        │   WAITING   │ ←── Users deposit USDC
        │             │     (Any time before start)
        └─────────────┘
               ↓
          start_run()
               ↓
        ┌─────────────┐
        │   ACTIVE    │ ←── Trading happens off-chain
        │             │     Vote stats updated on-chain
        └─────────────┘
               ↓
         settle_run()
               ↓
        ┌─────────────┐
        │   SETTLED   │ ←── Users withdraw their shares
        │             │     (Proportional + bonus)
        └─────────────┘
```

### Detailed Flow

```
1. CREATE RUN
   Backend calls: create_run(min_deposit, max_deposit, max_participants)
   Creates: Run PDA, Run Vault PDA
   Status: Waiting
   
2. USERS DEPOSIT
   Users call: deposit(amount)
   Transfers: USDC from user → run vault
   Creates: UserParticipation PDA per user
   Updates: Run.total_deposited, Run.participant_count
   
3. START RUN
   Backend calls: start_run()
   Validates: Min participants reached
   Updates: Status → Active, started_at timestamp
   Off-chain: Backend begins executing trades
   
4. VOTING (OFF-CHAIN)
   Backend executes voting rounds
   After each vote: update_vote_stats(user, correct, total)
   Updates: UserParticipation.correct_votes, total_votes
   
5. SETTLE RUN
   Backend calls: settle_run(final_balance)
   Validates: Vault balance matches
   Calculates: Platform fee (% of profit)
   Transfers: Fee from run vault → platform fee vault
   Updates: Status → Settled, final_balance, ended_at
   
6. USERS WITHDRAW
   Users call: withdraw()
   Calculates: Base share + bonus (from vote accuracy)
   Transfers: USDC from run vault → user wallet
   Updates: UserParticipation.withdrawn = true
   Special: Last user gets all remaining (fixes rounding)
```

---

## 💰 Money Flow

### Deposit Flow

```
┌─────────────┐                              ┌─────────────┐
│    User     │                              │  Run Vault  │
│   Wallet    │                              │   (PDA)     │
└─────────────┘                              └─────────────┘
      │                                             ▲
      │  deposit(amount)                            │
      │────────────────────────────────────────────►│
      │  Transfer: 100 USDC                         │
      │                                             │
      │  UserParticipation created                  │
      │  ├─ deposit_amount: 100 USDC                │
      │  ├─ withdrawn: false                        │
      │  └─ final_share: 0                          │
      │                                             │
      │  Run updated                                │
      │  ├─ total_deposited += 100                  │
      │  └─ participant_count += 1                  │
      └─────────────────────────────────────────────┘
```

### Settlement Flow

```
┌─────────────┐         ┌─────────────┐         ┌─────────────┐
│  Run Vault  │         │  Platform   │         │    Run      │
│   (1100)    │         │ Fee Vault   │         │   Account   │
└─────────────┘         └─────────────┘         └─────────────┘
      │                       ▲                         │
      │                       │                         │
      │  settle_run()         │                         │
      │  ├─ total_deposited: 1000 USDC                 │
      │  ├─ final_balance: 1100 USDC                   │
      │  ├─ profit: 100 USDC                           │
      │  ├─ platform_fee (15%): 15 USDC                │
      │  └─ available: 1085 USDC                       │
      │                       │                         │
      │  Transfer: 15 USDC    │                         │
      ├──────────────────────►│                         │
      │                       │                         │
      │                    Run.final_balance = 1085    │
      │                       │                         │
      │                    Run.platform_fee_amount = 15│
      └───────────────────────┴─────────────────────────┘
```

### Withdrawal Flow (With Bonus)

```
Example: User deposited 100 USDC of 1000 total
         Run ended with 1085 USDC available (after fee)
         User had 10/12 correct votes

┌─────────────────────────────────────────────────────────┐
│                    WITHDRAWAL CALCULATION                │
└─────────────────────────────────────────────────────────┘

Step 1: Base Share
base_share = (deposit / total_deposited) × final_balance
           = (100 / 1000) × 1085
           = 108.5 USDC

Step 2: Profit Share
profit = final_balance - total_deposited
       = 1085 - 1000
       = 85 USDC

user_profit_share = (deposit / total_deposited) × profit
                  = (100 / 1000) × 85
                  = 8.5 USDC

Step 3: Bonus (1% per correct vote)
bonus_rate = correct_votes × 1%
           = 10 × 1%
           = 10%

bonus = user_profit_share × bonus_rate
      = 8.5 × 10%
      = 0.85 USDC

Step 4: Final Share
final_share = base_share + bonus
            = 108.5 + 0.85
            = 109.35 USDC

┌─────────────┐                              ┌─────────────┐
│  Run Vault  │                              │    User     │
│             │                              │   Wallet    │
└─────────────┘                              └─────────────┘
      │                                             ▲
      │  withdraw()                                 │
      ├────────────────────────────────────────────►│
      │  Transfer: 109.35 USDC                      │
      │                                             │
      │  UserParticipation updated                  │
      │  ├─ final_share: 109.35                     │
      │  └─ withdrawn: true                         │
      │                                             │
      │  Run updated                                │
      │  ├─ total_withdrawn += 109.35               │
      │  └─ withdrawn_count += 1                    │
      └─────────────────────────────────────────────┘
```

---

## 🔐 Access Control Matrix

| Instruction | User | Platform Authority | Notes |
|-------------|------|-------------------|-------|
| `initialize_platform` | ❌ | ✅ | One-time setup |
| `create_run` | ❌ | ✅ | Creates new trading run |
| `create_run_vault` | ❌ | ✅ | Must be called after create_run |
| `deposit` | ✅ | ❌ | Users join run |
| `start_run` | ❌ | ✅ | Begin trading phase |
| `update_vote_stats` | ❌ | ✅ | Called by backend after each vote |
| `settle_run` | ❌ | ✅ | End run, distribute shares |
| `withdraw` | ✅ | ❌ | Users claim their share |
| `pause_platform` | ❌ | ✅ | Emergency stop |
| `unpause_platform` | ❌ | ✅ | Resume operations |
| `emergency_withdraw` | ❌ | ✅ | Requires platform paused |
| `withdraw_platform_fees` | ❌ | ✅ | Platform collects revenue |

---

## 🧮 Economic Formulas

### Platform Fee Calculation

```rust
profit = max(0, final_balance - total_deposited)
platform_fee = (profit × platform_fee_bps) / 10000
available_for_users = final_balance - platform_fee

// Example:
// total_deposited = 1000 USDC
// final_balance = 1200 USDC
// platform_fee_bps = 1500 (15%)
//
// profit = 1200 - 1000 = 200 USDC
// platform_fee = (200 × 1500) / 10000 = 30 USDC
// available = 1200 - 30 = 1170 USDC
```

### User Share Calculation

```rust
// Base share (proportional to deposit)
base_share = (user_deposit × final_balance) / total_deposited

// If profitable, calculate bonus
if final_balance > total_deposited {
    profit = final_balance - total_deposited
    user_profit_share = (user_deposit × profit) / total_deposited
    
    bonus_rate = correct_votes × 100 // 1% per vote in basis points
    bonus = (user_profit_share × bonus_rate) / 10000
    
    final_share = base_share + bonus
} else {
    // No bonus on losses
    final_share = base_share
}

// Special case: Last user gets all remaining
if is_last_user {
    final_share = vault.amount // Fixes rounding dust
}
```

### Maximum Bonus Impact

```
Max votes: 12
Bonus per vote: 1%
Max total bonus: 12%

Example with 100 USDC deposit, 10 USDC profit:
- User profit share: (100/1000) × 100 = 10 USDC
- Max bonus: 10 × 12% = 1.2 USDC
- Impact: 1.2% of total deposit (small but meaningful)
```

---

## 🗃️ Data Dependencies

### To Create a Run

```
Required:
├─ Platform must be initialized
├─ Platform must not be paused
└─ Parameters: min_deposit, max_deposit, max_participants

Creates:
├─ Run PDA (stores configuration and state)
└─ Run Vault PDA (holds USDC during run)
```

### To Deposit

```
Required:
├─ Run must exist
├─ Run status must be Waiting
├─ Platform must not be paused
├─ Amount must be between min_deposit and max_deposit
├─ Run must not be full
└─ User must have USDC balance

Creates:
└─ UserParticipation PDA (tracks user's involvement)

Updates:
├─ Run.total_deposited
└─ Run.participant_count
```

### To Settle

```
Required:
├─ Run must exist
├─ Run status must be Active
├─ Caller must be platform authority
├─ Vault balance must match reported final_balance
└─ Minimum run duration must have elapsed

Updates:
├─ Run.status → Settled
├─ Run.final_balance (after fee deduction)
├─ Run.platform_fee_amount
└─ Run.ended_at
```

### To Withdraw

```
Required:
├─ Run must exist
├─ Run status must be Settled
├─ UserParticipation must exist
├─ User must not have withdrawn already
└─ Vault must have sufficient balance

Updates:
├─ UserParticipation.final_share
├─ UserParticipation.withdrawn = true
├─ Run.total_withdrawn
└─ Run.withdrawn_count
```

---

## 🔍 PDA Derivation Reference

### Platform PDA

```rust
seeds = [b"platform"]
bump = platform.bump

// TypeScript
const [platformPda, platformBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform")],
    program.programId
);
```

### Platform Fee Vault PDA

```rust
seeds = [b"platform_fee_vault"]

// TypeScript
const [feeVaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("platform_fee_vault")],
    program.programId
);
```

### Run PDA

```rust
seeds = [b"run", run_id.to_le_bytes()]
bump = run.bump

// TypeScript
const runIdBuffer = Buffer.alloc(8);
runIdBuffer.writeBigUInt64LE(BigInt(runId));

const [runPda, runBump] = PublicKey.findProgramAddressSync(
    [Buffer.from("run"), runIdBuffer],
    program.programId
);
```

### Run Vault PDA

```rust
seeds = [b"vault", run_id.to_le_bytes()]

// TypeScript
const [vaultPda] = PublicKey.findProgramAddressSync(
    [Buffer.from("vault"), runIdBuffer],
    program.programId
);
```

### User Participation PDA

```rust
seeds = [b"participation", run_id.to_le_bytes(), user.key()]
bump = user_participation.bump

// TypeScript
const [participationPda, participationBump] = PublicKey.findProgramAddressSync(
    [
        Buffer.from("participation"),
        runIdBuffer,
        userPublicKey.toBuffer()
    ],
    program.programId
);
```

---

## 📊 Sample Data Flow

### Example Run from Start to Finish

```
INITIAL STATE
═════════════════════════════════════════════════════════════
Platform:
  - authority: 7xKx...
  - platform_fee_bps: 1500 (15%)
  - total_runs: 0
  - is_paused: false

Platform Fee Vault:
  - balance: 0 USDC

─────────────────────────────────────────────────────────────

STEP 1: CREATE RUN (Backend)
═════════════════════════════════════════════════════════════
create_run(
  run_id: 1,
  min_deposit: 10 USDC,
  max_deposit: 100 USDC,
  max_participants: 100
)

Result:
Run #1 created:
  - status: Waiting
  - total_deposited: 0
  - participant_count: 0
  - created_at: 1730000000

Run #1 Vault:
  - balance: 0 USDC

─────────────────────────────────────────────────────────────

STEP 2: USERS DEPOSIT
═════════════════════════════════════════════════════════════
User A: deposit(50 USDC)
User B: deposit(75 USDC)
User C: deposit(100 USDC)
User D: deposit(80 USDC)
User E: deposit(95 USDC)

Result:
Run #1:
  - total_deposited: 400 USDC
  - participant_count: 5

Run #1 Vault:
  - balance: 400 USDC

Participations created:
  - User A: 50 USDC deposited
  - User B: 75 USDC deposited
  - User C: 100 USDC deposited
  - User D: 80 USDC deposited
  - User E: 95 USDC deposited

─────────────────────────────────────────────────────────────

STEP 3: START RUN (Backend)
═════════════════════════════════════════════════════════════
start_run(run_id: 1)

Result:
Run #1:
  - status: Active
  - started_at: 1730003600

─────────────────────────────────────────────────────────────

STEP 4: TRADING & VOTING (Off-chain + vote updates)
═════════════════════════════════════════════════════════════
12 voting rounds occur...

Backend calls update_vote_stats after each round:
  - User A: 10/12 correct
  - User B: 11/12 correct
  - User C: 9/12 correct
  - User D: 12/12 correct (perfect!)
  - User E: 8/12 correct

Trading results in 25% profit!

─────────────────────────────────────────────────────────────

STEP 5: SETTLE RUN (Backend)
═════════════════════════════════════════════════════════════
settle_run(
  run_id: 1,
  final_balance: 500 USDC  // 25% profit
)

Calculations:
  profit = 500 - 400 = 100 USDC
  platform_fee = 100 × 15% = 15 USDC
  available = 500 - 15 = 485 USDC

Result:
Run #1:
  - status: Settled
  - final_balance: 485 USDC (after fee)
  - platform_fee_amount: 15 USDC
  - ended_at: 1730010800

Run #1 Vault:
  - balance: 485 USDC (15 USDC transferred to fee vault)

Platform Fee Vault:
  - balance: 15 USDC

─────────────────────────────────────────────────────────────

STEP 6: USERS WITHDRAW
═════════════════════════════════════════════════════════════

User A Withdrawal:
  base_share = (50/400) × 485 = 60.625 USDC
  profit_share = (50/400) × 100 = 12.5 USDC
  bonus = 12.5 × 10% = 1.25 USDC
  final_share = 60.625 + 1.25 = 61.875 USDC ✅

User B Withdrawal:
  base_share = (75/400) × 485 = 90.9375 USDC
  profit_share = (75/400) × 100 = 18.75 USDC
  bonus = 18.75 × 11% = 2.0625 USDC
  final_share = 90.9375 + 2.0625 = 93 USDC ✅

User C Withdrawal:
  base_share = (100/400) × 485 = 121.25 USDC
  profit_share = (100/400) × 100 = 25 USDC
  bonus = 25 × 9% = 2.25 USDC
  final_share = 121.25 + 2.25 = 123.5 USDC ✅

User D Withdrawal:
  base_share = (80/400) × 485 = 97 USDC
  profit_share = (80/400) × 100 = 20 USDC
  bonus = 20 × 12% = 2.4 USDC
  final_share = 97 + 2.4 = 99.4 USDC ✅

User E Withdrawal (LAST):
  Calculated: ~94 USDC
  Actual: 107.225 USDC (all remaining) ✅
  (Includes rounding dust)

Final State:
Run #1 Vault:
  - balance: 0 USDC (empty ✅)

All users withdrew successfully!
Platform collected 15 USDC fee!

═════════════════════════════════════════════════════════════
```

---

## 🎯 Key Invariants

The program must always maintain these invariants:

```
1. DEPOSIT INVARIANT
   ∑(all UserParticipation.deposit_amount) == Run.total_deposited

2. VAULT BALANCE INVARIANT (during run)
   Run.vault.amount == Run.total_deposited

3. VAULT BALANCE INVARIANT (after settlement)
   Run.vault.amount == Run.final_balance - Run.total_withdrawn

4. WITHDRAWAL INVARIANT
   ∑(all UserParticipation.final_share) <= Run.final_balance

5. PARTICIPANT COUNT INVARIANT
   count(UserParticipation accounts) == Run.participant_count

6. PLATFORM FEE INVARIANT
   Run.platform_fee_amount <= (Run.final_balance - Run.total_deposited)

7. ROUNDING INVARIANT
   After last withdrawal: Run.vault.amount == 0

8. BONUS INVARIANT
   user_bonus <= (user_profit_share × 12%)
```

---

## 🛠️ Integration Points

### Backend Service Must:

1. **Monitor** for deposits and track when min participants reached
2. **Call** `start_run` when ready to begin trading
3. **Execute** voting rounds off-chain
4. **Call** `update_vote_stats` after each round
5. **Trade** on Drift Protocol based on voting results
6. **Calculate** final balance accurately
7. **Call** `settle_run` with correct final balance
8. **Store** run results in database for frontend display

### Frontend Must:

1. **Connect** user wallet (Phantom, Solflare, etc.)
2. **Display** available runs with deposit limits
3. **Enable** USDC deposit transaction
4. **Show** run status (Waiting, Active, Settled)
5. **Display** user's vote statistics during run
6. **Show** estimated payout before settlement
7. **Enable** withdrawal transaction after settlement
8. **Display** transaction history

---

## 📈 Scalability Considerations

### Current Limits

```
Max participants per run: 1000 (configurable)
Max concurrent runs: Unlimited
Account sizes: Fixed (efficient)
Transaction costs: ~0.00001 SOL per operation
```

### Bottlenecks

```
❌ update_vote_stats requires 1 tx per user per round
   → 1000 users × 12 rounds = 12,000 transactions
   → At 400ms per tx = ~80 minutes
   
Solution: Batch updates or store votes off-chain
```

### Optimization Opportunities

```
1. Batch vote updates (remaining accounts pattern)
2. Use merkle proofs for large participant counts
3. Compress participant data with zero-copy
4. Store historical data off-chain (Arweave, etc.)
```

---

This architecture supports the current MVP and can scale to thousands of users with minor optimizations.


