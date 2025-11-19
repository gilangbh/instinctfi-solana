#!/bin/bash

# Script to verify program IDs match between Solana project and backend

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Program ID Verification${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Path to backend (adjust if needed)
BACKEND_PATH="../instinctfi-backend"

# 1. Check Anchor.toml
echo -e "${YELLOW}📋 Checking Anchor.toml...${NC}"
ANCHOR_PROGRAM_ID=$(grep -A 1 "\[programs.devnet\]" Anchor.toml | grep "instinct_trading" | cut -d '"' -f 2)
echo -e "Anchor.toml (devnet): ${GREEN}$ANCHOR_PROGRAM_ID${NC}"
echo ""

# 2. Check lib.rs declare_id
echo -e "${YELLOW}📋 Checking lib.rs declare_id...${NC}"
LIB_PROGRAM_ID=$(grep "declare_id!" programs/solana-program/src/lib.rs | cut -d '"' -f 2)
echo -e "lib.rs declare_id:    ${GREEN}$LIB_PROGRAM_ID${NC}"
echo ""

# 3. Check deployed program keypair (if exists)
if [ -f "target/deploy/instinct_trading-keypair.json" ]; then
    echo -e "${YELLOW}📋 Checking deployed program keypair...${NC}"
    KEYPAIR_PROGRAM_ID=$(solana-keygen pubkey target/deploy/instinct_trading-keypair.json)
    echo -e "Keypair pubkey:       ${GREEN}$KEYPAIR_PROGRAM_ID${NC}"
    echo ""
fi

# 4. Check IDL file
if [ -f "target/idl/instinct_trading.json" ]; then
    echo -e "${YELLOW}📋 Checking IDL file...${NC}"
    IDL_PROGRAM_ID=$(grep -o '"address": "[^"]*"' target/idl/instinct_trading.json | cut -d '"' -f 4)
    echo -e "IDL address:          ${GREEN}$IDL_PROGRAM_ID${NC}"
    echo ""
fi

# 5. Check backend .env (if exists)
if [ -f "$BACKEND_PATH/.env" ]; then
    echo -e "${YELLOW}📋 Checking backend .env...${NC}"
    BACKEND_ENV_ID=$(grep "^SOLANA_PROGRAM_ID=" "$BACKEND_PATH/.env" | cut -d '=' -f 2)
    echo -e "Backend .env:         ${GREEN}$BACKEND_ENV_ID${NC}"
    echo ""
fi

# 6. Check backend config.ts
if [ -f "$BACKEND_PATH/src/utils/config.ts" ]; then
    echo -e "${YELLOW}📋 Checking backend config.ts...${NC}"
    BACKEND_CONFIG_ID=$(grep "programId:" "$BACKEND_PATH/src/utils/config.ts" | grep -o "'[^']*'" | tail -1 | tr -d "'")
    echo -e "Backend config.ts:    ${GREEN}$BACKEND_CONFIG_ID${NC}"
    echo ""
fi

# Verification
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Verification Results${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Compare all IDs
ALL_MATCH=true

if [ "$ANCHOR_PROGRAM_ID" != "$LIB_PROGRAM_ID" ]; then
    echo -e "${RED}✗ MISMATCH: Anchor.toml and lib.rs don't match!${NC}"
    ALL_MATCH=false
fi

if [ -f "target/deploy/instinct_trading-keypair.json" ] && [ "$ANCHOR_PROGRAM_ID" != "$KEYPAIR_PROGRAM_ID" ]; then
    echo -e "${RED}✗ MISMATCH: Anchor.toml and keypair don't match!${NC}"
    ALL_MATCH=false
fi

if [ -f "target/idl/instinct_trading.json" ] && [ "$ANCHOR_PROGRAM_ID" != "$IDL_PROGRAM_ID" ]; then
    echo -e "${YELLOW}⚠ WARNING: Anchor.toml and IDL don't match (updating IDL...)${NC}"
    # Update IDL file
    sed -i.bak "s/\"address\": \"[^\"]*\"/\"address\": \"$ANCHOR_PROGRAM_ID\"/" target/idl/instinct_trading.json
    echo -e "${GREEN}✓ IDL updated to match Anchor.toml${NC}"
fi

if [ -f "$BACKEND_PATH/.env" ] && [ "$ANCHOR_PROGRAM_ID" != "$BACKEND_ENV_ID" ]; then
    echo -e "${RED}✗ MISMATCH: Backend .env doesn't match Solana project!${NC}"
    echo -e "${YELLOW}  Update backend .env to: SOLANA_PROGRAM_ID=$ANCHOR_PROGRAM_ID${NC}"
    ALL_MATCH=false
fi

if [ -f "$BACKEND_PATH/src/utils/config.ts" ] && [ "$ANCHOR_PROGRAM_ID" != "$BACKEND_CONFIG_ID" ]; then
    echo -e "${YELLOW}⚠ Backend config.ts fallback doesn't match${NC}"
fi

echo ""

if [ "$ALL_MATCH" = true ]; then
    echo -e "${GREEN}✓ All program IDs match! 🎉${NC}"
    echo -e "${GREEN}✓ Your Solana project and backend are in sync${NC}"
else
    echo -e "${RED}✗ Some program IDs don't match${NC}"
    echo -e "${YELLOW}  Fix the mismatches above before deploying${NC}"
fi

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Quick Reference${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Program ID: ${GREEN}$ANCHOR_PROGRAM_ID${NC}"
echo ""
echo "Check on Solana devnet:"
echo "  solana program show $ANCHOR_PROGRAM_ID --url devnet"
echo ""
echo "View in Explorer:"
echo "  https://explorer.solana.com/address/$ANCHOR_PROGRAM_ID?cluster=devnet"
echo ""
















