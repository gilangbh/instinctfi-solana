# Team Development Workflow

## 🎯 Overview

This document explains how multiple developers work together on the Instinct.fi Solana program.

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   SOLANA DEVNET                             │
│                                                             │
│   Program: 7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc     │
│   └── Deployed once, used by everyone                      │
└─────────────────────────────────────────────────────────────┘
            ▲                    ▲                    ▲
            │                    │                    │
    ┌───────┴──────┐     ┌───────┴──────┐    ┌───────┴──────┐
    │  Developer 1 │     │  Developer 2 │    │  Developer 3 │
    │              │     │              │    │              │
    │  Wallet A    │     │  Wallet B    │    │  Wallet C    │
    │  (private)   │     │  (private)   │    │  (private)   │
    └──────────────┘     └──────────────┘    └──────────────┘
```

## 🔑 Key Principles

### ✅ SHARE (in Git)
- **Program ID** - Everyone uses same devnet program
- **Source Code** - All Rust/JS code
- **IDL File** - Interface definition
- **Anchor.toml** - Configuration

### ❌ NEVER SHARE
- **Private Keys** - Each developer's wallet
- **Environment Variables** - API keys, secrets
- **Build Artifacts** - Generated .so files

## 👨‍💻 Roles & Responsibilities

### Lead Developer (Deploys to Devnet)
```bash
# Initial deployment
anchor build
anchor deploy --provider.cluster devnet

# After program changes
git commit -m "feat: added new feature"
anchor build
anchor deploy --provider.cluster devnet
git push
```

### Team Members (Use Existing Program)
```bash
# Just pull and use existing program ID
git pull
anchor test --skip-local-validator --provider.cluster devnet
```

## 📋 Daily Workflow

### For All Developers

#### Morning Routine
```bash
# 1. Pull latest code
git pull origin develop

# 2. Verify program is accessible
solana program show 7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc --url devnet

# 3. Get devnet SOL (if needed)
solana a找一个op 10 $(solana address) --url devnet
```

#### During Development
```bash
# 1. Make code changes
vim programs/solana-program/src/lib.rs

# 2. Build locally
anchor build

# 3. Run tests (uses your personal wallet)
anchor test --skip-local-validator --provider.cluster devnet

# 4. Commit if tests pass
git add .
git commit -m "feat: describe your change"
git push origin develop
```

### For Lead Developer Only

#### When Program Logic Changes
```bash
# 1. After merging PR with code changes
git pull origin develop

# 2. Build
anchor build

# 3. Deploy to devnet (this updates the live program!)
anchor deploy --provider.cluster devnet

# 4. Verify deployment
solana program show 7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc --url devnet

# 5. Notify team in Slack/Discord
# "Program deployed! Pull latest and test."
```

## 🔐 Wallet Management

### Each Developer's Wallet

**Location**: `~/.config/solana/id.json`

**Purpose**:
- ✅ Sign test transactions
- ✅ Pay for test transactions
- ✅ Test user interactions
- ❌ NOT for production

**Important**:
- Each developer generates their own
- Never commit to git
- Never share with others
- Get devnet SOL via airdrop

### Shared Admin Wallet (Optional)

For backend integration, you might want a shared devnet admin wallet:

```bash
# Generate shared admin wallet
solana-keygen new -o admin-devnet-keypair.json

# Share this with team (password protected or in 1Password)
# Backend team uses this for admin operations
```

**⚠️ Security Note**: This is ONLY for devnet testing. For mainnet, use a multisig wallet!

## 🌐 Environments

### Local Development (Fast Iteration)
```bash
# Terminal 1
solana-test-validator --reset

# Terminal 2
anchor test  # Points to local validator automatically
```

**Use when**:
- Fast iteration on program logic
- Testing without network latency
- No devnet SOL required

### Devnet (Shared Testing)
```bash
anchor test --skip-local-validator --provider.cluster devnet
```

**Use when**:
- Testing with real network conditions
- Multiple developers testing simultaneously
- Integration testing with backend

### Mainnet (Production)
```bash
anchor deploy --provider.cluster mainnet
```

**Use when**:
- ✅ Security audit complete
- ✅ All tests passing
- ✅ Ready for real users
- 🚨 Requires upgrade authority management!

## 🔄 Sync Workflow

### Scenario: Developer 2 Joins Team

```bash
# Day 1: Developer 2's first day

# 1. Clone repo
git clone <repo-url>
cd instinctfi-solana

# 2. Install dependencies
yarn install

# 3. Create personal wallet
solana-keygen new
# Creates ~/.config/solana/id.json

# 4. Fund wallet with devnet SOL
solana airdrop 10 $(solana address) --url devnet

# 5. Verify program exists
solana program show 7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc --url devnet

# 6. Build locally
anchor build

# 7. Run tests (uses their personal wallet!)
anchor test --skip-local-validator --provider.cluster devnet

# ✅ Ready to develop!
```

### Scenario: Program Updated by Lead

```bash
# Lead Developer
anchor build
anchor deploy --provider.cluster devnet
git push

# Other Developers
git pull  # Get latest code
anchor build  # Rebuild IDL
anchor test --skip-local-validator --provider.cluster devnet
# ✅ Now using updated program!
```

## 🏃 Backend Integration

### Backend Setup for Developers

Each backend developer:
```bash
# 1. Copy IDL to backend repo
cp target/idl/instinct_trading.json ../backend-repo/src/

# 2. Configure backend
echo "SOLANA_NETWORK=devnet" >> ../backend-repo/.env
echo "PROGRAM_ID=7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc" >> ../backend-repo/.env

# 3. Use their own backend wallet for testing
solana-keygen new -o ../backend-repo/.keys/backend-dev.json
```

## 🚨 Common Issues

### Issue: "Program not found"
**Solution**: Program needs to be deployed on devnet by lead developer

### Issue: "Insufficient funds"
**Solution**: 
```bash
solana airdrop 10 $(solana address) --url devnet
```

### Issue: "Tests fail with different errors"
**Solution**: Pull latest code - program may have been updated
```bash
git pull
anchor build
anchor test --skip-local-validator --provider.cluster devnet
```

### Issue: "Cannot upgrade program"
**Solution**: Only the original deployer (or someone with upgrade authority) can upgrade

### Issue: "Transaction simulation failed"
**Solution**: 
- Check devnet status: https://status.solana.com
- Try again in a few minutes (network congestion)

## 📊 Testing Strategy

### Unit Tests (Local)
```bash
# Start local validator
solana-test-validator --reset

# Run tests
anchor test
```

### Integration Tests (Devnet)
```bash
# Uses shared devnet program
anchor test --skip-local-validator --provider.cluster devnet
```

### E2E Tests (Devnet)
```bash
# Run with test script
npm test

# Or manually test via backend
curl -X POST http://localhost:3000/api/runs/create -d '{"minDeposit": 10, "maxDeposit": 100}'
```

## ✅ Best Practices

### DO
- ✅ Pull latest code before starting work
- ✅ Run tests before committing
- ✅ Use descriptive commit messages
- ✅ Deploy after significant changes
- ✅ Notify team when program is updated
- ✅ Keep `Anchor.toml` and `lib.rs` declare_id in sync

### DON'T
- ❌ Commit private keys
- ❌ Deploy every small change
- ❌ Modify program ID without coordinating
- ❌ Skip tests before deploying
- ❌ Work on outdated code

## 📞 Communication

### When to Notify Team

**Notify immediately**:
- Program deployed to devnet
- Breaking changes to program interface
- Critical bug fixes

**Update in PR**:
- New features
- Bug fixes
- Performance improvements

### Tools
- **Slack**: #instinct-dev channel
- **GitHub**: Pull requests for code review
- **Discord**: #devnet-testing for testing coordination


