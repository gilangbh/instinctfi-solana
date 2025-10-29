#!/bin/bash

# Setup script for new developers
# Run this when joining the team

set -e

echo "🚀 Setting up Instinct.fi development environment..."
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Check Anchor installation
echo -e "${BLUE}📦 Step 1: Checking Anchor installation...${NC}"
if ! command -v anchor &> /dev/null; then
    echo -e "${YELLOW}⚠️  Anchor not found. Installing...${NC}"
    npm install -g @coral-xyz/anchor-cli@0.31.1
else
    echo -e "${GREEN}✅ Anchor is installed${NC}"
fi

# Step 2: Check Solana CLI
echo -e "${BLUE}📦 Step 2: Checking Solana CLI...${NC}"
if ! command -v solana &> /dev/null; then
    echo -e "${YELLOW}⚠️  Solana CLI not found. Please install it:${NC}"
    echo "sh -c \"\$(curl -sSfL https://release.solana.com/v1.18.0/install)\""
    exit 1
else
    echo -e "${GREEN}✅ Solana CLI is installed${NC}"
fi

# Step 3: Configure Solana to use devnet
echo -e "${BLUE}🌐 Step 3: Configuring Solana for devnet...${NC}"
solana config set --url devnet

# Step 4: Check wallet
echo -e "${BLUE}🔑 Step 4: Setting up wallet...${NC}"
if [ ! -f ~/.config/solana/id.json ]; then
    echo -e "${YELLOW}⚠️  No wallet found. Creating new wallet...${NC}"
    solana-keygen new --no-bip39-passphrase
    echo -e "${GREEN}✅ Wallet created${NC}"
else
    echo -e "${GREEN}✅ Wallet already exists${NC}"
fi

WALLET_ADDRESS=$(solana address)
echo "Your wallet address: ${WALLET_ADDRESS}"

# Step 5: Get devnet SOL
echo -e "${BLUE}💰 Step 5: Getting devnet SOL...${NC}"
BALANCE=$(solana balance --url devnet | grep -oP '\d+\.?\d*')
echo "Current balance: ${BALANCE} SOL"

if (( $(echo "$BALANCE < 5" | bc -l) )); then
    echo -e "${YELLOW}⚠️  Balance is low. Requesting airdrop...${NC}"
    solana airdrop 10 ${WALLET_ADDRESS} --url devnet
    echo -e "${GREEN}✅ Airdrop received${NC}"
else
    echo -e "${GREEN}✅ You have enough SOL${NC}"
fi

# Step 6: Install dependencies
echo -e "${BLUE}📦 Step 6: Installing project dependencies...${NC}"
yarn install
echo -e "${GREEN}✅ Dependencies installed${NC}"

# Step 7: Check program on devnet
echo -e "${BLUE}🔍 Step 7: Checking program on devnet...${NC}"
PROGRAM_ID="7gmTYKqNX4xKsrd6NfNRscL3XSUoUTQyyTPhySWoABUc"

if solana program show ${PROGRAM_ID} --url devnet &> /dev/null; then
    echo -e "${GREEN}✅ Program found on devnet${NC}"
else
    echo -e "${YELLOW}⚠️  Program not found on devnet${NC}"
    echo "You may need to deploy it first, or ask the team lead to deploy"
fi

# Step 8: Build program
echo -e "${BLUE}🔨 Step 8: Building program...${NC}"
anchor build
echo -e "${GREEN}✅ Build complete${NC}"

# Step 9: Run tests
echo -e "${BLUE}🧪 Step 9: Running tests...${NC}"
anchor test --skip-local-validator --provider.cluster devnet || {
    echo -e "${YELLOW}⚠️  Tests failed, but setup is complete. Check the errors above.${NC}"
}

echo ""
echo -e "${GREEN}🎉 Setup complete!${NC}"
echo ""
echo "Summary:"
echo "--------"
echo "Wallet: ${WALLET_ADDRESS}"
echo "Cluster: devnet"
echo "Program: ${PROGRAM_ID}"
echo ""
echo "Next steps:"
echo "1. Read SETUP_GUIDE.md for detailed instructions"
echo "2. Pull latest code: git pull origin develop"
echo "3. Start developing!"
echo ""


