# Instinct.fi Solana Program

A gamified trading platform built on Solana where users vote collectively on perpetual futures trades.

## 🏗️ Architecture

This program implements a **hybrid approach**:
- **On-chain**: USDC deposit/withdrawal management, fund distribution, access control
- **Off-chain (backend)**: Voting logic, Drift Protocol integration, trade execution, timing

## 📋 Program Instructions

### Platform Management
- `initialize_platform` - One-time platform setup with fee configuration
- `pause_platform` - Emergency pause (admin only)
- `unpause_platform` - Resume operations (admin only)

### Run Management
- `create_run` - Create a new trading run
- `create_run_vault` - Initialize USDC vault for a run
- `start_run` - Start the run (moves from Waiting → Active)
- `settle_run` - End the run and record final P/L

### User Actions
- `deposit` - Join a run by depositing USDC
- `withdraw` - Claim your share after run settlement

### Backend Actions
- `update_vote_stats` - Update user's voting statistics
- `emergency_withdraw` - Emergency fund recovery (requires pause)

## 🧪 Testing

### Prerequisites
```bash
# Ensure you have:
- Solana CLI 1.18+
- Anchor CLI 0.31.1
- Node.js 16+
- Yarn
```

### Build the Program
```bash
cd solana-program
anchor build
```

### Run Tests
```bash
# Start local validator (in a separate terminal)
solana-test-validator

# Run all tests
anchor test --skip-local-validator

# Or test with fresh validator
anchor test
```

### Test Coverage
The test suite covers:
- ✅ Platform initialization
- ✅ Run creation and vault setup
- ✅ User deposits (valid and invalid amounts)
- ✅ Run lifecycle (waiting → active → settled)
- ✅ Withdrawals and share calculation
- ✅ Admin functions (pause/unpause)
- ✅ Error cases and validations

## 🔑 Key Concepts

### PDAs (Program Derived Addresses)
```
Platform:      ["platform"]
Run:           ["run", run_id]
Vault:         ["vault", run_id]
Participation: ["participation", run_id, user_pubkey]
```

### Run States
1. **Waiting** - Accepting deposits
2. **Active** - Trading in progress
3. **Settled** - Trading ended, ready for withdrawals

### Share Calculation
```
Base Share = (user_deposit / total_deposited) × final_balance
Bonus = correct_votes × 1% (max 12%)
User Payout = Base Share + Bonus
```

## 📊 Account Structures

### Platform
- `authority`: Admin public key
- `platform_fee_bps`: Fee in basis points (1500 = 15%)
- `total_runs`: Counter for runs created
- `is_paused`: Emergency pause flag

### Run
- `run_id`: Unique identifier
- `status`: Waiting | Active | Settled
- `total_deposited`: Total USDC deposited
- `final_balance`: Balance after trading
- `participant_count`: Number of participants
- `min/max_deposit`: Deposit limits
- `created_at`, `started_at`, `ended_at`: Timestamps

### UserParticipation
- `user`: User's public key
- `deposit_amount`: USDC deposited
- `final_share`: Amount withdrawn
- `correct_votes`: Bonus calculation
- `withdrawn`: Withdrawal status

## 🔒 Security Features

- ✅ **Access Control**: Authority checks on admin functions
- ✅ **Validation**: Deposit limits, status checks, balance verification
- ✅ **Reentrancy Protection**: Anchor's built-in guards
- ✅ **Emergency Controls**: Pause and emergency withdraw
- ✅ **Double-Withdrawal Prevention**: Tracks withdrawal status

## 🚀 Deployment

### Devnet
```bash
# Build
anchor build

# Deploy to devnet
anchor deploy --provider.cluster devnet

# Update program ID in Anchor.toml and lib.rs with new program ID
```

### Mainnet (After Audit)
```bash
# Deploy to mainnet
anchor deploy --provider.cluster mainnet

# Transfer upgrade authority (irreversible!)
solana program set-upgrade-authority <PROGRAM_ID> --final
```

## 📝 Next Steps

1. **Testing**: Complete comprehensive test coverage
2. **Security Audit**: Third-party audit before mainnet
3. **Backend Integration**: Build Node.js service to call these instructions
4. **Drift Integration**: Implement off-chain trading logic
5. **Monitoring**: Set up alerts for platform events

## 🤝 Integration with Backend

The backend should:
1. Call `create_run` and `create_run_vault` to start new runs
2. Monitor deposits and call `start_run` when ready
3. Execute trades on Drift Protocol based on voting
4. Call `update_vote_stats` after each voting round
5. Call `settle_run` with final P/L when run ends
6. Users call `withdraw` to claim their shares

## 📚 Resources

- [Anchor Documentation](https://www.anchor-lang.com/)
- [Solana Cookbook](https://solanacookbook.com/)
- [Drift Protocol Docs](https://docs.drift.trade/)

