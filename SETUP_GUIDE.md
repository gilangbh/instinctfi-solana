# Developer Setup Guide

## Quick Start for New Team Members

### 1. Clone Repository
```bash
git clone <your-repo-url>
cd instinctfi-solana
```

### 2. Install Dependencies
```bash
# Install Anchor
npm install -g @coral-xyz/anchor-cli@0.31.1

# Install Rust (if not already)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install project dependencies
yarn install
```

### 3. Setup Your Wallet (EACH DEVELOPER HAS DIFFERENT)
```bash
# Create your own devnet wallet (if you don't have one)
solana-keygen new

# Airdrop devnet SOL to your wallet
solana airdrop 10 $(solana address) --url devnet

# Verify your balance
solana balance --url devnet
```

### 4. Verify Program ID
```bash
# Check that the program is deployed on devnet
solana program show 7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc --url devnet

# Should show program info ✅
```

### 5. Run Tests
```bash
# First, copy the IDL file (this is already in git)
# If missing, someone needs to run `anchor build` first

# Run the tests
anchor test --skip-local-validator --provider.cluster devnet
```

### 6. Build (if making changes)
```bash
# Build the program locally
anchor build

# This generates target/idl/instinct_trading.json
# Commit this if it changes!
```

### 7. Deploy to Devnet (ONLY IF YOU'RE UPDATING THE PROGRAM)
```bash
# Check you have enough SOL
solana balance --url devnet

# Deploy (only if you have deployment authority)
anchor deploy --provider.cluster devnet

# Note: Only the person who originally deployed can upgrade without upgrade authority
```

## 🔐 Wallet Setup Explained

### Your Wallet Location
```
~/.config/solana/id.json  ← YOUR personal wallet (never share!)
```

This wallet is ONLY for:
- ✅ Testing transactions
- ✅ Paying transaction fees
- ✅ Signing test transactions

### What Happens in Practice

```
You                Teammate
│                  │
├── Wallet A       ├── Wallet B
└── Sign txns      └── Sign txns
     │                  │
     └──────┬───────────┘
            ▼
     Program: 7gmTYK... (SAME for everyone!)
```

Both of you interact with the **same deployed program**, but using **your own wallets**.

## 🚨 Important Notes

### DO NOT COMMIT
- `~/.config/solana/id.json` (private keys!)
- `.env` files with secrets
- `target/` directory (generated files)

### DO COMMIT
- `Anchor.toml` with program ID
- `programs/solana-program/src/lib.rs` with declare_id!
- `target/idl/instinct_trading.json` (Interface Definition)
- All source code

## 🔄 Development Workflow

### Daily Workflow
```bash
# 1. Pull latest code
git pull origin develop

# 2. Your wallet is already set up (one time)

# 3. Make changes to programs/solana-program/src/lib.rs

# 4. Build
anchor build

# 5. Test (uses your wallet)
anchor test --skip-local-validator --provider.cluster devnet

# 6. If tests pass, commit and push
git add .
git commit -m "Your changes"
git push origin develop
```

### When Program Logic Changes
Only ONE person needs to deploy (usually the lead developer):

```bash
# Lead developer runs:
anchor build
anchor deploy --provider.cluster devnet

# Everyone else just pulls the updated IDL
git pull
```

### Local Development (Faster Iteration)
For quick iteration without deploying to devnet:

```bash
# Terminal 1: Start local validator
solana-test-validator --reset

# Terminal 2: Run tests locally
anchor test

# Terminal 3: Your backend pointing to local
export SOLANA_NETWORK=localnet
# Backend uses http://127.0.0.1:8899
```

## 🏗️ For Backend Developers

### Setup Backend to Use Devnet Program

1. Copy IDL file:
```bash
cp target/idl/instinct_trading.json ../your-backend-repo/src/
```

2. Install dependencies:
```bash
cd ../your-backend-repo
npm install @coral-xyz/anchor @solana/web3.js
```

3. Use the program ID:
```typescript
const PROGRAM_ID = '7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc';
const RPC_URL = 'https://api.devnet.solana.com';

const program = new Program(IDL, PROGRAM_ID, {
  connection: new Connection(RPC_URL),
});
```

### Backend Environment Variables
```bash
# .env
SOLANA_NETWORK=devnet
SOLANA_RPC_URL=https://api.devnet.solana.com
INSTINCT_PROGRAM_ID=7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc
ADMIN_PRIVATE_KEY=[array of numbers from your admin wallet]
```

## ❓ Troubleshooting

### "Error: insufficient funds"
```bash
# Get more devnet SOL
solana airdrop 10 $(solana address) --url devnet
```

### "Error: custom program error"
- Program might need redeployment
- Check with lead developer if program was updated
- Pull latest code: `git pull`

### "Error: Signature verification failed"
- Your wallet config is incorrect
- Check: `solana address` matches your id.json

### Program not found
```bash
# Check if program exists on devnet
solana program show 7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc --url devnet

# If "Program not found", someone needs to deploy first
```

## 📞 Getting Help

- **Program issues**: Check Anchor docs
- **Solana issues**: Check Solana docs
- **Backend issues**: Check your backend repo README
- **Team issues**: Ask in team chat


