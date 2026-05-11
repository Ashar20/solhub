use anchor_lang::prelude::*;

declare_id!("Eemnq9Fv55B2TNi5zKNSQyQDd6CKFBUJMfcgtJUiJuFB");

#[program]
pub mod workflow_registry {
    use super::*;

    /// Register a new workflow on-chain.
    /// Owner signs. Steps are stored off-chain (IPFS/DB); only hash stored here.
    pub fn register_workflow(
        ctx: Context<RegisterWorkflow>,
        params: RegisterParams,
    ) -> Result<()> {
        let workflow = &mut ctx.accounts.workflow;
        workflow.owner = ctx.accounts.owner.key();
        workflow.name = params.name;
        workflow.trigger_type = params.trigger_type;
        workflow.steps_hash = params.steps_hash;
        workflow.steps_cid = params.steps_cid;
        workflow.is_active = true;
        workflow.execution_count = 0;
        workflow.created_at = Clock::get()?.unix_timestamp;
        workflow.last_executed_at = 0;
        workflow.platform_authority = ctx.accounts.platform_authority.key();
        workflow.bump = ctx.bumps.workflow;
        Ok(())
    }

    /// Toggle workflow active/inactive. Owner only.
    pub fn set_workflow_status(
        ctx: Context<SetStatus>,
        is_active: bool,
    ) -> Result<()> {
        ctx.accounts.workflow.is_active = is_active;
        Ok(())
    }

    /// Called by platform authority after each successful execution.
    /// Only the platform_authority stored in the workflow account may call this.
    pub fn record_execution(ctx: Context<RecordExecution>) -> Result<()> {
        let workflow = &mut ctx.accounts.workflow;
        workflow.execution_count = workflow
            .execution_count
            .checked_add(1)
            .ok_or(ErrorCode::Overflow)?;
        workflow.last_executed_at = Clock::get()?.unix_timestamp;
        Ok(())
    }
}

#[account]
pub struct WorkflowAccount {
    pub owner: Pubkey,             // 32
    pub name: String,              // 4 + 64
    pub trigger_type: u8,          // 1  (0=cron, 1=account_watch, 2=webhook)
    pub steps_hash: [u8; 32],      // 32 (SHA-256)
    pub steps_cid: String,         // 4 + 64 (IPFS CID)
    pub is_active: bool,           // 1
    pub execution_count: u64,      // 8
    pub created_at: i64,           // 8
    pub last_executed_at: i64,     // 8
    pub platform_authority: Pubkey, // 32
    pub bump: u8,                  // 1
}
// Space: 8 (discriminator) + 32 + 68 + 1 + 32 + 68 + 1 + 8 + 8 + 8 + 32 + 1 = 267 -> allocate 300

#[derive(Accounts)]
#[instruction(params: RegisterParams)]
pub struct RegisterWorkflow<'info> {
    #[account(
        init,
        payer = owner,
        space = 8 + 300,
        seeds = [b"workflow", owner.key().as_ref(), params.name.as_bytes()],
        bump
    )]
    pub workflow: Account<'info, WorkflowAccount>,
    #[account(mut)]
    pub owner: Signer<'info>,
    /// The platform authority that will be stored in the workflow for future record_execution calls.
    pub platform_authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetStatus<'info> {
    #[account(
        mut,
        has_one = owner,
        seeds = [b"workflow", owner.key().as_ref(), workflow.name.as_bytes()],
        bump = workflow.bump
    )]
    pub workflow: Account<'info, WorkflowAccount>,
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
pub struct RecordExecution<'info> {
    #[account(
        mut,
        has_one = platform_authority,
        seeds = [b"workflow", workflow.owner.as_ref(), workflow.name.as_bytes()],
        bump = workflow.bump
    )]
    pub workflow: Account<'info, WorkflowAccount>,
    pub platform_authority: Signer<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RegisterParams {
    pub name: String,
    pub trigger_type: u8,
    pub steps_hash: [u8; 32],
    pub steps_cid: String,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Arithmetic overflow")]
    Overflow,
}
