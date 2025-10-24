const anchor = require("@coral-xyz/anchor");
const { SystemProgram, Keypair, LAMPORTS_PER_SOL } = anchor.web3;
const { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } = require("@solana/spl-token");
const assert = require("assert");

describe("Instinct Trading", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.InstinctTrading;
  const payer = provider.wallet;

  // Test accounts
  let usdcMint;
  let platformAuthority;
  let platformPda;
  let user1;
  let user2;
  let user1TokenAccount;
  let user2TokenAccount;

  const PLATFORM_FEE_BPS = 1500; // 15%
  const RUN_ID = new anchor.BN(1);
  const MIN_DEPOSIT = new anchor.BN(10_000_000); // 10 USDC (6 decimals)
  const MAX_DEPOSIT = new anchor.BN(100_000_000); // 100 USDC
  const MAX_PARTICIPANTS = 100;

  before(async () => {
    // Generate test keypairs
    platformAuthority = Keypair.generate();
    user1 = Keypair.generate();
    user2 = Keypair.generate();

    // Airdrop SOL to test accounts
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        platformAuthority.publicKey,
        2 * LAMPORTS_PER_SOL
      )
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(user1.publicKey, 2 * LAMPORTS_PER_SOL)
    );
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(user2.publicKey, 2 * LAMPORTS_PER_SOL)
    );

    // Create USDC mock mint
    usdcMint = await createMint(
      provider.connection,
      payer.payer,
      payer.publicKey,
      null,
      6 // USDC has 6 decimals
    );

    // Create token accounts for users
    user1TokenAccount = await createAccount(
      provider.connection,
      payer.payer,
      usdcMint,
      user1.publicKey
    );

    user2TokenAccount = await createAccount(
      provider.connection,
      payer.payer,
      usdcMint,
      user2.publicKey
    );

    // Mint USDC to users
    await mintTo(
      provider.connection,
      payer.payer,
      usdcMint,
      user1TokenAccount,
      payer.publicKey,
      1000_000_000 // 1000 USDC
    );

    await mintTo(
      provider.connection,
      payer.payer,
      usdcMint,
      user2TokenAccount,
      payer.publicKey,
      1000_000_000 // 1000 USDC
    );

    console.log("✅ Test setup complete");
    console.log("USDC Mint:", usdcMint.toString());
    console.log("Platform Authority:", platformAuthority.publicKey.toString());
    console.log("User 1:", user1.publicKey.toString());
    console.log("User 2:", user2.publicKey.toString());
  });

  describe("Platform Initialization", () => {
    it("Initializes the platform", async () => {
      [platformPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("platform")],
        program.programId
      );

      const tx = await program.methods
        .initializePlatform(PLATFORM_FEE_BPS)
        .accounts({
          platform: platformPda,
          authority: platformAuthority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([platformAuthority])
        .rpc();

      console.log("Platform initialized:", tx);

      // Verify platform state
      const platform = await program.account.platform.fetch(platformPda);
      assert.equal(platform.authority.toString(), platformAuthority.publicKey.toString());
      assert.equal(platform.platformFeeBps, PLATFORM_FEE_BPS);
      assert.equal(platform.totalRuns.toNumber(), 0);
      assert.equal(platform.isPaused, false);
    });

    it("Fails to initialize platform twice", async () => {
      try {
        await program.methods
          .initializePlatform(PLATFORM_FEE_BPS)
          .accounts({
            platform: platformPda,
            authority: platformAuthority.publicKey,
            systemProgram: SystemProgram.programId,
          })
          .signers([platformAuthority])
          .rpc();
        assert.fail("Should have thrown error");
      } catch (err) {
        assert.ok(err.toString().includes("already in use"));
      }
    });
  });

  describe("Run Management", () => {
    let runPda;
    let runVaultPda;

    it("Creates a new run", async () => {
      [runPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("run"), RUN_ID.toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      const tx = await program.methods
        .createRun(RUN_ID, MIN_DEPOSIT, MAX_DEPOSIT, MAX_PARTICIPANTS)
        .accounts({
          platform: platformPda,
          run: runPda,
          authority: platformAuthority.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .signers([platformAuthority])
        .rpc();

      console.log("Run created:", tx);

      // Verify run state
      const run = await program.account.run.fetch(runPda);
      assert.equal(run.runId.toNumber(), RUN_ID.toNumber());
      assert.equal(run.minDeposit.toNumber(), MIN_DEPOSIT.toNumber());
      assert.equal(run.maxDeposit.toNumber(), MAX_DEPOSIT.toNumber());
      assert.equal(run.maxParticipants, MAX_PARTICIPANTS);
      assert.equal(run.participantCount, 0);
      assert.equal(run.totalDeposited.toNumber(), 0);

      // Verify platform total runs increased
      const platform = await program.account.platform.fetch(platformPda);
      assert.equal(platform.totalRuns.toNumber(), 1);
    });

    it("Creates vault for the run", async () => {
      [runVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), RUN_ID.toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      const tx = await program.methods
        .createRunVault(RUN_ID)
        .accounts({
          run: runPda,
          runVault: runVaultPda,
          usdcMint: usdcMint,
          payer: platformAuthority.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([platformAuthority])
        .rpc();

      console.log("Vault created:", tx);

      // Verify vault exists
      const vaultAccount = await getAccount(provider.connection, runVaultPda);
      assert.equal(vaultAccount.mint.toString(), usdcMint.toString());
      assert.equal(vaultAccount.owner.toString(), runPda.toString());
    });
  });

  describe("User Deposits", () => {
    let runPda;
    let runVaultPda;
    let user1ParticipationPda;

    before(() => {
      [runPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("run"), RUN_ID.toArrayLike(Buffer, "le", 8)],
        program.programId
      );
      [runVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), RUN_ID.toArrayLike(Buffer, "le", 8)],
        program.programId
      );
    });

    it("User 1 deposits USDC", async () => {
      const depositAmount = new anchor.BN(50_000_000); // 50 USDC

      [user1ParticipationPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("participation"),
          RUN_ID.toArrayLike(Buffer, "le", 8),
          user1.publicKey.toBuffer(),
        ],
        program.programId
      );

      const tx = await program.methods
        .deposit(RUN_ID, depositAmount)
        .accounts({
          platform: platformPda,
          run: runPda,
          userParticipation: user1ParticipationPda,
          runVault: runVaultPda,
          userTokenAccount: user1TokenAccount,
          usdcMint: usdcMint,
          user: user1.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user1])
        .rpc();

      console.log("User 1 deposited:", tx);

      // Verify participation state
      const participation = await program.account.userParticipation.fetch(
        user1ParticipationPda
      );
      assert.equal(participation.user.toString(), user1.publicKey.toString());
      assert.equal(participation.depositAmount.toNumber(), depositAmount.toNumber());
      assert.equal(participation.withdrawn, false);

      // Verify vault balance
      const vaultAccount = await getAccount(provider.connection, runVaultPda);
      assert.equal(vaultAccount.amount.toString(), depositAmount.toString());

      // Verify run updated
      const run = await program.account.run.fetch(runPda);
      assert.equal(run.participantCount, 1);
      assert.equal(run.totalDeposited.toNumber(), depositAmount.toNumber());
    });

    it("Fails deposit below minimum", async () => {
      const lowAmount = new anchor.BN(1_000_000); // 1 USDC

      const [user2ParticipationPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("participation"),
          RUN_ID.toArrayLike(Buffer, "le", 8),
          user2.publicKey.toBuffer(),
        ],
        program.programId
      );

      try {
        await program.methods
          .deposit(RUN_ID, lowAmount)
          .accounts({
            platform: platformPda,
            run: runPda,
            userParticipation: user2ParticipationPda,
            runVault: runVaultPda,
            userTokenAccount: user2TokenAccount,
            usdcMint: usdcMint,
            user: user2.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([user2])
          .rpc();
        assert.fail("Should have thrown error");
      } catch (err) {
        assert.ok(err.toString().includes("DepositTooLow"));
      }
    });
  });

  describe("Run Lifecycle", () => {
    let runPda;

    before(() => {
      [runPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("run"), RUN_ID.toArrayLike(Buffer, "le", 8)],
        program.programId
      );
    });

    it("Starts the run", async () => {
      const tx = await program.methods
        .startRun(RUN_ID)
        .accounts({
          platform: platformPda,
          run: runPda,
          authority: platformAuthority.publicKey,
        })
        .signers([platformAuthority])
        .rpc();

      console.log("Run started:", tx);

      // Verify run status changed
      const run = await program.account.run.fetch(runPda);
      assert.equal(run.status.active !== undefined, true);
      assert.ok(run.startedAt.toNumber() > 0);
    });

    it("Settles the run", async () => {
      const [runVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), RUN_ID.toArrayLike(Buffer, "le", 8)],
        program.programId
      );

      // For testing, final balance = initial deposit (no profit/loss)
      const finalBalance = new anchor.BN(50_000_000);

      // Get current run to see participant count
      const run = await program.account.run.fetch(runPda);
      
      // Create participant shares array matching participant count
      const participantShares = [
        {
          user: user1.publicKey,
          shareAmount: finalBalance,
        }
      ];

      const tx = await program.methods
        .settleRun(RUN_ID, finalBalance, participantShares)
        .accounts({
          platform: platformPda,
          run: runPda,
          runVault: runVaultPda,
          authority: platformAuthority.publicKey,
        })
        .signers([platformAuthority])
        .rpc();

      console.log("Run settled:", tx);

      // Verify run status
      const updatedRun = await program.account.run.fetch(runPda);
      assert.equal(updatedRun.status.settled !== undefined, true);
      assert.equal(updatedRun.finalBalance.toNumber(), finalBalance.toNumber());
      assert.ok(updatedRun.endedAt.toNumber() > 0);
    });
  });

  describe("Withdrawals", () => {
    let runPda;
    let runVaultPda;
    let user1ParticipationPda;

    before(() => {
      [runPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("run"), RUN_ID.toArrayLike(Buffer, "le", 8)],
        program.programId
      );
      [runVaultPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("vault"), RUN_ID.toArrayLike(Buffer, "le", 8)],
        program.programId
      );
      [user1ParticipationPda] = anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("participation"),
          RUN_ID.toArrayLike(Buffer, "le", 8),
          user1.publicKey.toBuffer(),
        ],
        program.programId
      );
    });

    it("User withdraws funds", async () => {
      const beforeBalance = await getAccount(provider.connection, user1TokenAccount);

      const tx = await program.methods
        .withdraw(RUN_ID)
        .accounts({
          run: runPda,
          userParticipation: user1ParticipationPda,
          runVault: runVaultPda,
          userTokenAccount: user1TokenAccount,
          user: user1.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([user1])
        .rpc();

      console.log("User withdrew:", tx);

      // Verify participation updated
      const participation = await program.account.userParticipation.fetch(
        user1ParticipationPda
      );
      assert.equal(participation.withdrawn, true);
      assert.ok(participation.finalShare.toNumber() > 0);

      // Verify user received funds
      const afterBalance = await getAccount(provider.connection, user1TokenAccount);
      assert.ok(afterBalance.amount > beforeBalance.amount);
    });

    it("Fails to withdraw twice", async () => {
      try {
        await program.methods
          .withdraw(RUN_ID)
          .accounts({
            run: runPda,
            userParticipation: user1ParticipationPda,
            runVault: runVaultPda,
            userTokenAccount: user1TokenAccount,
            user: user1.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([user1])
          .rpc();
        assert.fail("Should have thrown error");
      } catch (err) {
        assert.ok(err.toString().includes("AlreadyWithdrawn"));
      }
    });
  });

  describe("Admin Functions", () => {
    it("Pauses the platform", async () => {
      const tx = await program.methods
        .pausePlatform()
        .accounts({
          platform: platformPda,
          authority: platformAuthority.publicKey,
        })
        .signers([platformAuthority])
        .rpc();

      console.log("Platform paused:", tx);

      const platform = await program.account.platform.fetch(platformPda);
      assert.equal(platform.isPaused, true);
    });

    it("Unpauses the platform", async () => {
      const tx = await program.methods
        .unpausePlatform()
        .accounts({
          platform: platformPda,
          authority: platformAuthority.publicKey,
        })
        .signers([platformAuthority])
        .rpc();

      console.log("Platform unpaused:", tx);

      const platform = await program.account.platform.fetch(platformPda);
      assert.equal(platform.isPaused, false);
    });
  });
});

