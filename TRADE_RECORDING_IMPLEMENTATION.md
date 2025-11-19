# Trade Recording On-Chain Implementation

## ✅ Completed

### 1. Solana Program (`lib.rs`)
- ✅ Added `TradeRecord` account structure
- ✅ Added `TradeDirection` enum (Long, Short, Skip)
- ✅ Added `record_trade` instruction
- ✅ Added `RecordTrade` context struct
- ✅ Added error codes: `InvalidRunId`, `InvalidLeverage`, `InvalidPositionSize`
- ✅ Fixed missing closing brace in `settle_run`

### 2. Backend Service (`SolanaService.ts`)
- ✅ Added `getTradeRecordPDA()` method
- ✅ Added `recordTrade()` method with full parameter conversion
- ✅ Handles price conversion (USDC → micro-USDC)
- ✅ Handles leverage conversion (decimal → integer, e.g., 2.6x → 26)
- ✅ Handles position size conversion (decimal → integer, e.g., 96.1% → 96)
- ✅ Handles signed PnL (negative values)

### 3. Run Service Integration (`RunService.ts`)
- ✅ Calls `recordTrade()` after executing trades (when position opens)
- ✅ Logs trade closure (ready for future `update_trade` instruction)
- ✅ Non-blocking - trades continue even if on-chain recording fails

## ⚠️ Pending Tasks

### 1. Get Actual Discriminator
**Location**: `SolanaService.ts` line 366

The discriminator for `record_trade` is currently a placeholder:
```typescript
const discriminator = Buffer.from([0, 0, 0, 0, 0, 0, 0, 0]); // PLACEHOLDER
```

**How to get the actual discriminator:**
1. Build the Solana program: `anchor build`
2. Check the IDL file: `target/idl/instinct_trading.json`
3. Find the `record_trade` instruction and its discriminator
4. Or use Anchor's instruction discriminator calculation:
   ```bash
   anchor idl parse --file target/idl/instinct_trading.json
   ```

**Alternative**: Use Anchor's program interface instead of manual transaction building (requires IDL setup).

### 2. Add `update_trade` Instruction (Optional)
Currently, trades are recorded when they open, but exit price and PnL are not updated on-chain when positions close.

**To implement:**
1. Add `update_trade` instruction to `lib.rs`:
   ```rust
   pub fn update_trade(
       ctx: Context<UpdateTrade>,
       run_id: u64,
       round: u8,
       exit_price: u64,
       pnl: i64,
   ) -> Result<()> {
       // Update existing TradeRecord
   }
   ```

2. Add `UpdateTrade` context struct
3. Add method to `SolanaService.ts`
4. Call it from `RunService.closePosition()`

## 📋 Data Format

### TradeRecord Account
- `run_id`: u64
- `round`: u8 (1-12)
- `direction`: TradeDirection (0=Long, 1=Short, 2=Skip)
- `entry_price`: u64 (micro-USDC, 6 decimals)
- `exit_price`: u64 (micro-USDC, 0 if still open)
- `pnl`: i64 (micro-USDC, can be negative, 0 if still open)
- `leverage`: u8 (10 = 1.0x, 20 = 2.0x, 26 = 2.6x, etc.)
- `position_size_percent`: u8 (10-100)
- `executed_at`: i64 (Unix timestamp)
- `bump`: u8

### PDA Seeds
```
["trade", run_id (8 bytes), round (1 byte)]
```

## 🔍 Testing

### 1. Build and Deploy Program
```bash
cd programs/solana-program
anchor build
anchor deploy
```

### 2. Get Discriminator
After building, check the IDL or use:
```bash
anchor idl parse --file target/idl/instinct_trading.json | grep record_trade
```

### 3. Test Trade Recording
1. Create a run
2. Start the run
3. Execute a trade (should trigger `recordTrade()`)
4. Check Solana Explorer for the TradeRecord account

## 📝 Notes

- **Non-blocking**: Trade recording failures don't stop trading
- **Price Format**: All prices stored in micro-USDC (multiply by 1,000,000)
- **Leverage Format**: Stored as integer (multiply by 10)
- **Position Size Format**: Stored as integer (round to nearest whole number)
- **PnL Format**: Signed integer in micro-USDC (can be negative)

## 🚀 Next Steps

1. **Get discriminator** from compiled program/IDL
2. **Test** trade recording end-to-end
3. **Optional**: Add `update_trade` instruction for closing positions
4. **Optional**: Add trade verification in `settle_run` to verify final balance matches recorded trades

