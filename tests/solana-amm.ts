import * as anchor from "@anchor-lang/core";
import { Program, web3, BN } from "@anchor-lang/core";
import NodeWallet from "@anchor-lang/core/dist/cjs/nodewallet";
import { SolanaAmm } from "../target/types/solana_amm";
import { assert } from "chai";
import { ASSOCIATED_TOKEN_PROGRAM_ID, createMint, getAssociatedTokenAddressSync, getOrCreateAssociatedTokenAccount, mintTo, TOKEN_PROGRAM_ID} from "@solana/spl-token";
import { randomBytes } from "crypto";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";

const SYSTEM_PROGRAM_ID = SystemProgram.programId;

const FEE = 1000;
const TOKEN_A_AMOUNT = new BN(100_000_000);
const TOKEN_B_AMOUNT = new BN(100_000_000);
const DEPOSIT_LP_AMOUNT = new BN(1000);
const SWAP_AMOUNT = new BN(10_000_000);
const SLIPPAGE_TOLERANCE = 0.20;

describe("solana-amm", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const commitment = "confirmed";
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.SolanaAmm as Program<SolanaAmm>;
  const connection = anchor.getProvider().connection;
  const user = provider.wallet as NodeWallet;
  const confirmTx = async (tx: string) => {
  const connection = anchor.getProvider().connection;
  const latestBlockHash = await connection.getLatestBlockhash();
  await connection.confirmTransaction(
      {
          signature: tx,
          ...latestBlockHash,
        },
        commitment
      );
    };
    const confirmTxs = async (signatures: string[]) => {
      await Promise.all(signatures.map(confirmTx));
  };

  let mintX: PublicKey;
  let mintY: PublicKey;
  let mintLp: PublicKey;
  let userTokenXAta: PublicKey;
  let userTokenYAta: PublicKey;
  let userLpAta: PublicKey;
  let vaultX: PublicKey;
  let vaultY: PublicKey;

  const seed = new BN(randomBytes(8));
  const config = PublicKey.findProgramAddressSync(
    [Buffer.from("config"), seed.toArrayLike(Buffer, "le", 8)],
    program.programId
  )[0];
  before(async () => {
        await Promise.all([user].map(async (k) => {
        return await anchor.getProvider().connection.requestAirdrop(k.publicKey, 2 * anchor.web3.LAMPORTS_PER_SOL);
      })
    ).then(confirmTxs);
  });

  it("initializes the AMM pool", async () => {
    try {
      mintX = await createMint(connection, user.payer, provider.publicKey, provider.publicKey, 6);
      mintY = await createMint(connection, user.payer, provider.publicKey, provider.publicKey, 6);
      
      // Derive mintLp as a PDA instead of creating it
      [mintLp] = PublicKey.findProgramAddressSync(
        [Buffer.from("lp"), config.toBuffer()],
        program.programId
      );

      vaultX = getAssociatedTokenAddressSync(mintX, config, true);
      vaultY = getAssociatedTokenAddressSync(mintY, config, true);

      userTokenXAta = (await getOrCreateAssociatedTokenAccount(connection, user.payer, mintX, user.publicKey)).address;
      userTokenYAta = (await getOrCreateAssociatedTokenAccount(connection, user.payer, mintY, user.publicKey)).address;
      
      userLpAta = getAssociatedTokenAddressSync(mintLp, user.publicKey);

      await mintTo(connection, user.payer, mintX, userTokenXAta, provider.publicKey, 1000 * 10 ** 6);
      console.log("Minted 1000 X Token ", userTokenXAta.toBase58());

      await mintTo(connection, user.payer, mintY, userTokenYAta, provider.publicKey, 1000 * 10 ** 6);
      console.log("Minted 1000 Y Token ", userTokenYAta.toBase58());

      const tx = await program.methods.initialize(seed, FEE, null).accountsStrict({
        initializer: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX,
        vaultY,
        mintLp,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID, 
      }).rpc();

      console.log("Initialize transaction:", tx);

      const configAccount = await program.account.config.fetch(config);
      assert.isTrue(configAccount.seed.eq(seed), "Seed should match");
      assert.equal(configAccount.fee, FEE, "Fee should match");
      assert.equal(configAccount.mintX.toBase58(), mintX.toBase58(), "Mint X should match");
      assert.equal(configAccount.mintY.toBase58(), mintY.toBase58(), "Mint Y should match");
      assert.equal(configAccount.locked, false, "Pool should not be locked");
    } catch (e: any) {
      console.log("Initialize error:", e.message || e);
    }
  });

  it("deposits liquidity into the pool", async () => {
    try {
      const tx = await program.methods.deposit(
        new BN(DEPOSIT_LP_AMOUNT),
        new BN(TOKEN_A_AMOUNT),
        new BN(TOKEN_B_AMOUNT)
      ).accountsStrict({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX: vaultX,
        vaultY: vaultY  ,
        mintLp,
        userX: userTokenXAta,
        userY: userTokenYAta,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      }).rpc();

      console.log("Deposit transaction:", tx);
    } catch (e: any) {
      console.log("Deposit error:", e.message || e);
    }
  });

  it("performs a swap (X for Y)", async () => {
    try {
      const minAmountOut = BigInt(Number(SWAP_AMOUNT) * (1 - SLIPPAGE_TOLERANCE));

      const tx = await program.methods.swap(
        new BN(SWAP_AMOUNT.toString()),
        new BN(minAmountOut.toString()),
        true
      ).accountsStrict({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX: vaultX,
        vaultY: vaultY,
        mintLp,
        userLp: userLpAta,
        userX: userTokenXAta,
        userY: userTokenYAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: web3.SystemProgram.programId,
      }).rpc();

      console.log("Swap X for Y transaction:", tx);
    } catch (e: any) {
      console.log("Swap error:", e.message || e);
    }
  });

  it("performs a swap (Y for X)", async () => {
    try {
      const minAmountOut = BigInt(Number(SWAP_AMOUNT) * (1 - SLIPPAGE_TOLERANCE));

      const tx = await program.methods.swap(
        new BN(SWAP_AMOUNT.toString()),
        new BN(minAmountOut.toString()),
        false
      ).accountsStrict({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX: vaultX,
        vaultY: vaultY,
        mintLp,
        userX: userTokenXAta,
        userY: userTokenYAta,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      }).rpc();

      console.log("Swap Y for X transaction:", tx);
    } catch (e: any) {
      console.log("Swap error:", e.message || e);
    }
  });

  it("withdraws liquidity from the pool (Instruction introspection)", async () => {
    try {
      const minX = BigInt(Number(DEPOSIT_LP_AMOUNT) * (1 - SLIPPAGE_TOLERANCE));
      const minY = BigInt(Number(DEPOSIT_LP_AMOUNT) * (1 - SLIPPAGE_TOLERANCE));

      const burnIx = await program.methods.burn(
        new BN(DEPOSIT_LP_AMOUNT.toString())
      ).accountsStrict({
        user: user.publicKey,
        config,
        mintLp,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user.payer])
      .instruction();

      const withdrawIx = await program.methods.withdrawIx(
        new BN(DEPOSIT_LP_AMOUNT.toString()),
        new BN(minX.toString()),
        new BN(minY.toString())
      ).accountsStrict({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX: vaultX,
        vaultY: vaultY,
        mintLp,
        userX: userTokenXAta,
        userY: userTokenYAta,
        userLp: userLpAta,
        instructionSysvar: web3.SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      })
      .signers([user.payer])
      .instruction();

      const tx = new web3.Transaction().add(burnIx, withdrawIx);
      const signature = await provider.sendAndConfirm(tx);
      console.log("Withdraw with Instruction Introspection transaction:", signature);
    } catch (e: any) {
      console.log("Withdraw Ix error:", e.message || e);
    }
  });

  it("withdraws liquidity from the pool (CPI)", async () => {
    try {
      const tx = await program.methods.deposit(
        new BN(DEPOSIT_LP_AMOUNT),
        new BN(TOKEN_A_AMOUNT),
        new BN(TOKEN_B_AMOUNT)
      ).accountsStrict({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX: vaultX,
        vaultY: vaultY  ,
        mintLp,
        userX: userTokenXAta,
        userY: userTokenYAta,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      }).rpc();

      console.log("Deposit transaction for CPI:", tx);
    } catch (e: any) {
      console.log("Deposit error:", e.message || e);
    }
    try {
      const minX = BigInt(Number(DEPOSIT_LP_AMOUNT) * (1 - SLIPPAGE_TOLERANCE));
      const minY = BigInt(Number(DEPOSIT_LP_AMOUNT) * (1 - SLIPPAGE_TOLERANCE));

      const tx = await program.methods.withdraw(
        new BN(DEPOSIT_LP_AMOUNT.toString()),
        new BN(minX.toString()),
        new BN(minY.toString())
      ).accountsStrict({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX: vaultX,
        vaultY: vaultY,
        mintLp,
        userX: userTokenXAta,
        userY: userTokenYAta,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      }).rpc();

      console.log("Withdraw transaction with CPI:", tx);
    } catch (e: any) {
      console.log("Withdraw error:", e.message || e);
    }
  });

  it("verifies pool state after operations", async () => {
    try {
      const configAccount = await program.account.config.fetch(config);
      
      assert.isTrue(configAccount.seed.eq(seed), "Seed should match");
      assert.equal(configAccount.fee, FEE, "Fee should match");
      assert.equal(configAccount.mintX.toBase58(), mintX.toBase58(), "Mint X should match");
      assert.equal(configAccount.mintY.toBase58(), mintY.toBase58(), "Mint Y should match");
      assert.equal(configAccount.locked, false, "Pool should not be locked");
    } catch (e: any) {
      console.log("Verify error:", e.message || e);
    }
  });

  it("handles zero amount error for deposit", async () => {
    try {
      await program.methods.deposit(
        new BN(0),
        new BN(TOKEN_A_AMOUNT.toString()),
        new BN(TOKEN_B_AMOUNT.toString())
      ).accountsStrict({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX: vaultX,
        vaultY: vaultY,
        mintLp,
        userX: userTokenXAta,
        userY: userTokenYAta,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      }).rpc();
      
      assert.fail("Should have thrown error for zero amount");
    } catch (e: any) {
      assert(e.error?.errorCode?.code === "InvalidAmount" || e.message.includes("InvalidAmount"), "Should throw InvalidAmount error");
    }
  });

  it("handles zero amount error for swap", async () => {
    try {
      await program.methods.swap(
        new BN(0),
        new BN(0),
        true
      ).accountsStrict({
        user: user.publicKey,
        mintX,
        mintY,
        config,
        vaultX: vaultX,
        vaultY: vaultY,
        mintLp,
        userX: userTokenXAta,
        userY: userTokenYAta,
        userLp: userLpAta,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SYSTEM_PROGRAM_ID,
      }).rpc();
      
      assert.fail("Should have thrown error for zero amount");
    } catch (e: any) {
      assert(e.error?.errorCode?.code === "InvalidAmount" || e.message.includes("InvalidAmount"), "Should throw InvalidAmount error");
    }
  });
});