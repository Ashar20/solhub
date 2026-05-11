use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("4CFgDzuLnfdTThgNXTknhXyshzsidDQFtNCxsoMnBHJn");

#[program]
pub mod execution_vault {
    use super::*;

    /// Caller deposits USDC credits into their org vault.
    /// Initialises the vault PDA on first call.
    pub fn deposit_credits(ctx: Context<DepositCredits>, amount: u64) -> Result<()> {
        // Initialise org_id on first deposit
        if ctx.accounts.vault.org_id == Pubkey::default() {
            ctx.accounts.vault.org_id = ctx.accounts.depositor.key();
            ctx.accounts.vault.bump = ctx.bumps.vault;
        }
        token::transfer(ctx.accounts.into_transfer_context(), amount)?;
        ctx.accounts.vault.credits = ctx
            .accounts
            .vault
            .credits
            .checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;
        Ok(())
    }

    /// Platform deducts fee per execution.
    /// 80% goes to creator_account balance, 20% conceptually stays in treasury.
    pub fn debit_execution(
        ctx: Context<DebitExecution>,
        fee_usdc: u64,
    ) -> Result<()> {
        let vault = &mut ctx.accounts.caller_vault;
        require!(vault.credits >= fee_usdc, ErrorCode::InsufficientCredits);
        vault.credits -= fee_usdc;

        let creator_share = fee_usdc * 80 / 100;
        let _treasury_share = fee_usdc - creator_share;

        // Initialise creator account on first debit for this creator
        if ctx.accounts.creator_account.owner == Pubkey::default() {
            ctx.accounts.creator_account.owner = ctx.accounts.creator.key();
            ctx.accounts.creator_account.bump = ctx.bumps.creator_account;
        }

        ctx.accounts.creator_account.balance = ctx
            .accounts
            .creator_account
            .balance
            .checked_add(creator_share)
            .ok_or(ErrorCode::Overflow)?;
        ctx.accounts.creator_account.total_earned = ctx
            .accounts
            .creator_account
            .total_earned
            .checked_add(creator_share)
            .ok_or(ErrorCode::Overflow)?;

        emit!(ExecutionBilled {
            workflow: ctx.accounts.workflow.key(),
            fee_usdc,
            creator_share,
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    /// Creator withdraws their accumulated balance to their token account.
    pub fn withdraw_creator(ctx: Context<WithdrawCreator>, amount: u64) -> Result<()> {
        let creator = &mut ctx.accounts.creator_account;
        require!(creator.balance >= amount, ErrorCode::InsufficientBalance);
        creator.balance -= amount;

        let vault_bump = ctx.accounts.vault_pda.bump;
        let owner_key = ctx.accounts.owner.key();
        let seeds: &[&[u8]] = &[b"vault", owner_key.as_ref(), &[vault_bump]];
        let signer_seeds = &[seeds];
        let cpi_ctx = ctx.accounts.into_transfer_context_with_signer(signer_seeds);
        token::transfer(cpi_ctx, amount)?;
        Ok(())
    }
}

// ─── Account Structs ──────────────────────────────────────────────────────────

#[account]
pub struct VaultAccount {
    pub org_id: Pubkey,   // 32
    pub credits: u64,     // 8 — USDC in base units (6 decimals)
    pub total_spent: u64, // 8
    pub bump: u8,         // 1
}
// Space: 8 (discriminator) + 32 + 8 + 8 + 1 = 57 -> allocate 64

#[account]
pub struct CreatorAccount {
    pub owner: Pubkey,     // 32
    pub balance: u64,      // 8
    pub total_earned: u64, // 8
    pub bump: u8,          // 1
}
// Space: 8 (discriminator) + 32 + 8 + 8 + 1 = 57 -> allocate 64

// ─── Events ───────────────────────────────────────────────────────────────────

#[event]
pub struct ExecutionBilled {
    pub workflow: Pubkey,
    pub fee_usdc: u64,
    pub creator_share: u64,
    pub timestamp: i64,
}

// ─── Accounts Contexts ────────────────────────────────────────────────────────

#[derive(Accounts)]
pub struct DepositCredits<'info> {
    #[account(
        init_if_needed,
        payer = depositor,
        space = 8 + 64,
        seeds = [b"vault", depositor.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, VaultAccount>,

    /// Depositor's SPL token account (source of USDC).
    #[account(mut)]
    pub depositor_token_account: Account<'info, TokenAccount>,

    /// The vault's SPL token account (destination).
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub depositor: Signer<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> DepositCredits<'info> {
    pub fn into_transfer_context(&self) -> CpiContext<'_, '_, '_, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.depositor_token_account.to_account_info(),
            to: self.vault_token_account.to_account_info(),
            authority: self.depositor.to_account_info(),
        };
        CpiContext::new(self.token_program.to_account_info(), cpi_accounts)
    }
}

#[derive(Accounts)]
#[instruction(fee_usdc: u64)]
pub struct DebitExecution<'info> {
    #[account(
        mut,
        seeds = [b"vault", caller_vault.org_id.as_ref()],
        bump = caller_vault.bump
    )]
    pub caller_vault: Account<'info, VaultAccount>,

    #[account(
        init_if_needed,
        payer = platform_authority,
        space = 8 + 64,
        seeds = [b"creator", creator.key().as_ref()],
        bump
    )]
    pub creator_account: Account<'info, CreatorAccount>,

    /// The creator wallet (used for PDA seed derivation).
    /// CHECK: only used as a seed; not signing this call
    pub creator: UncheckedAccount<'info>,

    /// CHECK: workflow account referenced for event emission only
    pub workflow: UncheckedAccount<'info>,

    #[account(mut)]
    pub platform_authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WithdrawCreator<'info> {
    #[account(
        mut,
        has_one = owner,
        seeds = [b"creator", owner.key().as_ref()],
        bump = creator_account.bump
    )]
    pub creator_account: Account<'info, CreatorAccount>,

    /// Creator's SPL token account (destination).
    #[account(mut)]
    pub creator_token_account: Account<'info, TokenAccount>,

    /// Vault's SPL token account that holds the USDC funds (source).
    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,

    /// The vault PDA — signs the SPL transfer out.
    #[account(
        seeds = [b"vault", owner.key().as_ref()],
        bump = vault_pda.bump
    )]
    pub vault_pda: Account<'info, VaultAccount>,

    pub owner: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

impl<'info> WithdrawCreator<'info> {
    pub fn into_transfer_context_with_signer<'a>(
        &self,
        signer_seeds: &'a [&'a [&'a [u8]]],
    ) -> CpiContext<'a, 'a, 'a, 'info, Transfer<'info>> {
        let cpi_accounts = Transfer {
            from: self.vault_token_account.to_account_info(),
            to: self.creator_token_account.to_account_info(),
            authority: self.vault_pda.to_account_info(),
        };
        CpiContext::new_with_signer(self.token_program.to_account_info(), cpi_accounts, signer_seeds)
    }
}

// ─── Errors ───────────────────────────────────────────────────────────────────

#[error_code]
pub enum ErrorCode {
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Insufficient credits in vault")]
    InsufficientCredits,
    #[msg("Insufficient creator balance")]
    InsufficientBalance,
}
