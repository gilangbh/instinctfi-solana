# Security Fixes Checklist

**Use this checklist to track implementation of audit findings**

---

## 🔴 CRITICAL FIXES (BLOCKING)

### Fix #1: Platform Fee Implementation
- [ ] Add `total_fees_collected` and `platform_fee_vault` to Platform struct
- [ ] Update `Platform::LEN` constant
- [ ] Create platform fee vault in `initialize_platform`
- [ ] Calculate platform fee in `settle_run` (profit only, not principal)
- [ ] Transfer fee to platform vault during settlement
- [ ] Add `platform_fee_amount` field to Run struct
- [ ] Update `Run::LEN` constant
- [ ] Implement `withdraw_platform_fees` instruction
- [ ] Add test: platform fee collected on profitable run
- [ ] Add test: no platform fee on losing run
- [ ] Add test: platform can withdraw collected fees
- [ ] Verify in logs: fee amount is correct
- [ ] Update Anchor.toml if account sizes changed

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Fix #2: Withdrawal Rounding Fix
- [ ] Add `total_withdrawn` field to Run struct
- [ ] Add `withdrawn_count` field to Run struct
- [ ] Update `Run::LEN` constant
- [ ] Implement "last user gets remaining" logic in `withdraw`
- [ ] Calculate `is_last_user` = (withdrawn_count + 1 == participant_count)
- [ ] Track withdrawals: increment `withdrawn_count` after each withdrawal
- [ ] Track withdrawals: accumulate `total_withdrawn` amount
- [ ] Add checked arithmetic (no unwrap!)
- [ ] Add test: 3 users with odd final balance (e.g., 31 USDC)
- [ ] Add test: verify last user gets exact remaining amount
- [ ] Add test: verify vault is empty after all withdrawals
- [ ] Add test: large participant count (100 users)
- [ ] Fuzz test: random deposits and final balances

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Fix #3: Bonus Calculation Fix
- [ ] Modify bonus to apply only to profit portion, not principal
- [ ] Calculate profit: `final_balance - total_deposited` (if positive)
- [ ] Calculate user's profit share: `(deposit * profit) / total_deposited`
- [ ] Apply bonus to profit share only: `bonus = profit_share * correct_votes%`
- [ ] No bonus if run lost money (final_balance <= total_deposited)
- [ ] Add overflow checks for all calculations
- [ ] Add test: bonus with profit (verify math)
- [ ] Add test: no bonus on losing run
- [ ] Add test: all users with 12/12 votes (edge case)
- [ ] Add test: mixed vote scores (realistic scenario)
- [ ] Verify: sum of all withdrawals <= final_balance

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

## 🟠 HIGH PRIORITY FIXES

### Fix #4: Vote Stats Validation
- [ ] Add constant `MAX_VOTES = 12`
- [ ] Validate `total_votes <= MAX_VOTES` in `update_vote_stats`
- [ ] Validate `correct_votes <= total_votes` in `update_vote_stats`
- [ ] Validate `new_total >= old_total` (prevent decreases)
- [ ] Add `ErrorCode::InvalidVoteCount`
- [ ] Add `ErrorCode::VoteCountDecreased`
- [ ] Add test: reject vote count > 12
- [ ] Add test: reject correct > total
- [ ] Add test: reject decreasing vote count
- [ ] Add test: accept valid incremental updates

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Fix #5: Settlement Validation
- [ ] Add constant `MIN_RUN_DURATION = 3600` (1 hour)
- [ ] Validate run duration in `settle_run`
- [ ] Check: `current_time >= started_at + MIN_RUN_DURATION`
- [ ] Add `ErrorCode::RunTooShort`
- [ ] Add test: reject settlement before minimum duration
- [ ] Add test: accept settlement after duration
- [ ] Consider: Add configurable duration per run (optional)

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Fix #6: Deposit Deadline
- [ ] Add `deposit_deadline` field to Run struct (i64 timestamp)
- [ ] Update `Run::LEN` constant
- [ ] Set deadline in `create_run` (e.g., start_time - 1 hour)
- [ ] Validate deadline in `deposit` instruction
- [ ] Check: `current_time <= run.deposit_deadline`
- [ ] Add `ErrorCode::DepositsClosedForRun`
- [ ] Add test: reject deposit after deadline
- [ ] Add test: accept deposit before deadline
- [ ] Update frontend to show deadline

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Fix #7: Minimum Participants
- [ ] Add constant `MIN_PARTICIPANTS = 5` (or configurable)
- [ ] Add constant `MAX_PARTICIPANTS_LIMIT = 1000`
- [ ] Validate in `create_run`: `max_participants >= MIN_PARTICIPANTS`
- [ ] Validate in `create_run`: `max_participants <= MAX_PARTICIPANTS_LIMIT`
- [ ] Validate in `start_run`: `participant_count >= MIN_PARTICIPANTS`
- [ ] Add `ErrorCode::InsufficientParticipants`
- [ ] Update existing `ErrorCode::InvalidParticipantLimit` message
- [ ] Add test: reject run start with < MIN_PARTICIPANTS
- [ ] Add test: accept run start with >= MIN_PARTICIPANTS
- [ ] Add test: reject creation with max > 1000

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

## 🟡 MEDIUM PRIORITY FIXES

### Fix #8: Run Cancellation
- [ ] Add `Cancelled` variant to `RunStatus` enum
- [ ] Implement `cancel_run` instruction
- [ ] Validate: can only cancel if status == Waiting
- [ ] Validate: only authority can cancel
- [ ] Update Run status to Cancelled
- [ ] Add `ErrorCode::CannotCancelActiveRun`
- [ ] Implement `withdraw_from_cancelled_run` for refunds
- [ ] Users get exact deposit back (no calculations)
- [ ] Add test: cancel waiting run
- [ ] Add test: reject cancel active run
- [ ] Add test: users can withdraw deposits after cancel

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Fix #9: Emergency Withdraw Timelock
- [ ] Add `emergency_withdraw_delay` to Platform (e.g., 48 hours)
- [ ] Add `PendingWithdrawal` struct with run_id, amount, initiated_at
- [ ] Add `pending_withdrawal` Option field to Platform
- [ ] Implement `initiate_emergency_withdraw` instruction
- [ ] Store pending withdrawal with timestamp
- [ ] Emit event for transparency
- [ ] Implement `execute_emergency_withdraw` instruction
- [ ] Validate: timelock has expired
- [ ] Clear pending withdrawal after execution
- [ ] Add `ErrorCode::TimelockNotExpired`
- [ ] Add test: timelock prevents immediate withdrawal
- [ ] Add test: withdrawal succeeds after delay

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Fix #10: Maximum Total Deposit Cap
- [ ] Add `max_total_deposit` field to Run struct
- [ ] Update `Run::LEN` constant
- [ ] Accept `max_total_deposit` parameter in `create_run`
- [ ] Validate in `deposit`: `total_deposited + amount <= max_total_deposit`
- [ ] Add `ErrorCode::RunCapExceeded`
- [ ] Add test: reject deposit exceeding cap
- [ ] Add test: accept deposit within cap
- [ ] Frontend: show remaining capacity

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Fix #11: Structured Events
- [ ] Define `#[event]` structs for key operations
- [ ] Add `RunCreated` event
- [ ] Add `RunStarted` event
- [ ] Add `RunSettled` event
- [ ] Add `UserDeposited` event
- [ ] Add `UserWithdrew` event
- [ ] Add `PlatformFeeCollected` event
- [ ] Add `EmergencyWithdrawInitiated` event
- [ ] Emit events in respective instructions
- [ ] Add test: verify events are emitted
- [ ] Document event schema for indexers

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Fix #12: Update Platform Fee Mechanism
- [ ] Implement `update_platform_fee` instruction
- [ ] Validate: new fee <= MAX_PLATFORM_FEE_BPS (2000 = 20%)
- [ ] Validate: only authority can update
- [ ] Add `PlatformFeeUpdated` event
- [ ] Add test: authority can update fee
- [ ] Add test: non-authority cannot update
- [ ] Add test: reject fee > 20%

**Assigned to**: _____________  
**Target date**: _____________  
**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

## 🔵 LOW PRIORITY / BEST PRACTICES

### Improvement #1: Use InitSpace
- [ ] Add `#[derive(InitSpace)]` to Platform struct
- [ ] Add `#[derive(InitSpace)]` to Run struct
- [ ] Add `#[derive(InitSpace)]` to UserParticipation struct
- [ ] Replace manual LEN with `8 + Platform::INIT_SPACE`
- [ ] Remove manual LEN constants
- [ ] Test: verify account sizes are correct

**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Improvement #2: Add Version Field
- [ ] Add `version: u8` to Platform struct
- [ ] Set to 1 during initialization
- [ ] Document: increment on breaking changes
- [ ] Use for migration logic in future

**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Improvement #3: Better Error Messages
- [ ] Review all ErrorCode messages
- [ ] Add context and suggestions
- [ ] Document error codes in README
- [ ] Add error code reference for frontend

**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Improvement #4: Constants File
- [ ] Create constants module
- [ ] Move all magic numbers to constants
- [ ] Document rationale for each constant
- [ ] Make configurable where appropriate

**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

### Improvement #5: Helper Methods
- [ ] Add `Run::is_accepting_deposits()` helper
- [ ] Add `Run::is_full()` helper
- [ ] Add `Run::can_start()` helper
- [ ] Add `Run::can_settle()` helper
- [ ] Use helpers in instruction validations

**Status**: ⬜ Not Started / 🟡 In Progress / ✅ Complete  

---

## 🧪 TESTING REQUIREMENTS

### Unit Tests
- [ ] Platform fee: collected on profit
- [ ] Platform fee: not collected on loss
- [ ] Withdrawal: last user gets remaining
- [ ] Withdrawal: no dust in vault
- [ ] Bonus: calculated correctly on profit
- [ ] Bonus: zero on loss
- [ ] Vote stats: validation (correct <= total)
- [ ] Vote stats: max 12 votes
- [ ] Vote stats: no decreases
- [ ] Settlement: minimum duration enforced
- [ ] Deposit: deadline enforced
- [ ] Start: minimum participants enforced
- [ ] Run: cancellation works correctly

**Target**: 50+ unit tests  
**Current**: _____  
**Status**: ⬜ Not Complete / ✅ Complete  

---

### Integration Tests
- [ ] Complete run lifecycle (create → deposit → start → settle → withdraw)
- [ ] Multiple runs in parallel
- [ ] Platform fee collection and withdrawal
- [ ] Edge case: 1 USDC deposits
- [ ] Edge case: Maximum u64 values
- [ ] Edge case: 1000 participants
- [ ] Race condition: simultaneous withdrawals
- [ ] Cancellation: full refund flow

**Target**: 10+ integration tests  
**Current**: _____  
**Status**: ⬜ Not Complete / ✅ Complete  

---

### Attack Scenario Tests
- [ ] Reproduce: Bonus vault drain attack
- [ ] Reproduce: Rounding dust accumulation
- [ ] Reproduce: Platform fee circumvention (should be fixed)
- [ ] Reproduce: Vote manipulation
- [ ] Reproduce: Front-running deposits
- [ ] Reproduce: Single user run
- [ ] Reproduce: Emergency rug pull (with timelock)
- [ ] Verify: All attacks are mitigated

**Target**: 7 attack scenario tests  
**Current**: _____  
**Status**: ⬜ Not Complete / ✅ Complete  

---

### Fuzz Tests
- [ ] Set up cargo-fuzz or similar
- [ ] Fuzz: random deposits and final balances
- [ ] Fuzz: random vote distributions
- [ ] Fuzz: random participant counts
- [ ] Invariant: sum(withdrawals) == final_balance
- [ ] Invariant: vault.amount >= remaining_withdrawals
- [ ] Run for 1M+ iterations

**Status**: ⬜ Not Complete / ✅ Complete  

---

## 📋 CODE REVIEW CHECKLIST

### Code Quality
- [ ] All arithmetic uses checked operations (no unwrap on math)
- [ ] All error cases have specific error codes
- [ ] No TODO or FIXME comments remaining
- [ ] All functions documented
- [ ] Complex logic has explanatory comments

### Security
- [ ] All accounts validated (has_one, seeds, bump)
- [ ] Access control on all privileged operations
- [ ] No unsafe code
- [ ] No uninitialized memory access
- [ ] Reentrancy protections (Anchor handles this)

### Testing
- [ ] All new code has tests
- [ ] All changed code has updated tests
- [ ] Test coverage > 90%
- [ ] All tests passing locally
- [ ] All tests passing in CI

### Documentation
- [ ] README updated with changes
- [ ] API documentation updated
- [ ] Migration guide (if breaking changes)
- [ ] Changelog entry added

---

## 🚀 DEPLOYMENT CHECKLIST

### Pre-Devnet
- [ ] All critical fixes complete
- [ ] All high-priority fixes complete
- [ ] Code review passed
- [ ] All tests passing
- [ ] Build succeeds without warnings
- [ ] Update program ID in Anchor.toml and lib.rs

### Devnet Deployment
- [ ] Deploy to devnet: `anchor deploy --provider.cluster devnet`
- [ ] Initialize platform on devnet
- [ ] Create test run
- [ ] Test deposits from multiple wallets
- [ ] Test start run
- [ ] Test vote updates
- [ ] Test settlement with platform fee
- [ ] Test withdrawals (verify no dust)
- [ ] Monitor for issues (1 week)

### Pre-Mainnet
- [ ] Professional security audit completed
- [ ] All audit findings addressed
- [ ] Re-audit if critical changes made
- [ ] Bug bounty program run (2+ weeks)
- [ ] All bug bounty findings addressed
- [ ] Legal review completed
- [ ] Multi-sig set up (Squads or similar)
- [ ] Monitoring and alerts configured
- [ ] Emergency response plan documented
- [ ] Team trained on emergency procedures

### Mainnet Deployment
- [ ] Deploy to mainnet: `anchor deploy --provider.cluster mainnet`
- [ ] Initialize platform on mainnet
- [ ] Set conservative deposit caps initially (e.g., $10K per run)
- [ ] Create first production run
- [ ] Monitor 24/7 for first week
- [ ] Gradually increase caps over 2-4 weeks
- [ ] Full production after 1 month of smooth operation

---

## 📊 METRICS & MONITORING

### Development Metrics
- [ ] Test coverage: ___% (target: >90%)
- [ ] Critical issues: ___ (target: 0)
- [ ] High issues: ___ (target: 0)
- [ ] Medium issues: ___ (target: <3)
- [ ] Build time: ___s (target: <60s)

### Production Metrics (Post-Launch)
- [ ] Uptime: ___% (target: >99.9%)
- [ ] Successful withdrawals: ___% (target: 100%)
- [ ] Vault balance accuracy: ___% (target: 100%)
- [ ] Average withdrawal time: ___s (target: <30s)
- [ ] Platform fee collection rate: ___% (target: 100%)

### Alerts Configured
- [ ] Vault balance mismatch alert
- [ ] Failed withdrawal alert
- [ ] Platform fee not collected alert
- [ ] Unusual withdrawal pattern alert
- [ ] Emergency function called alert
- [ ] Authority key used alert

---

## 📝 SIGN-OFF

### Development Team
- [ ] Lead Developer: _____________ Date: _______
- [ ] Security Engineer: _____________ Date: _______
- [ ] QA Engineer: _____________ Date: _______

### Management
- [ ] CTO Approval: _____________ Date: _______
- [ ] CEO Approval: _____________ Date: _______

### External
- [ ] Security Auditor: _____________ Date: _______
- [ ] Legal Counsel: _____________ Date: _______

---

## 📞 SUPPORT CONTACTS

**Development Issues**:  
Lead Developer: _____________  
Email: _____________  
Telegram: _____________  

**Security Issues**:  
Security Engineer: _____________  
Email: _____________  
24/7 Hotline: _____________  

**Emergency Contacts**:  
On-call Rotation: _____________  
Escalation Path: _____________  

---

**Last Updated**: _____________  
**Next Review**: _____________  
**Version**: 1.0


