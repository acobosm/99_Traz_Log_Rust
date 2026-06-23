use anchor_lang::prelude::*;

declare_id!("HR3UbH45KuTanX5yiNDnPnYXr19r7mNuzUPPtj6acDJJ");

#[program]
pub mod blockchain {
    use super::*;

    pub fn initialize_institution(ctx: Context<InitializeInstitution>, name: String, pac_budget_limit: u64) -> Result<()> {
        let institution = &mut ctx.accounts.institution;
        institution.authority = ctx.accounts.authority.key();
        institution.name = name;
        institution.pac_budget_limit = pac_budget_limit;
        institution.pac_budget_spent = 0;
        
        msg!("Institucion inicializada: {} con limite PAC: {}", institution.name, institution.pac_budget_limit);
        Ok(())
    }

    pub fn initialize_process(
        ctx: Context<InitializeProcess>,
        id: u64,
        title: String,
        referential_budget: u64,
        deadline: i64,
        ipfs_tdr_hash: String,
    ) -> Result<()> {
        let institution = &mut ctx.accounts.institution;
        let process = &mut ctx.accounts.process;
        
        // 1. Validar límite presupuestario (PAC)
        let new_spent = institution.pac_budget_spent.checked_add(referential_budget)
            .ok_or(ErrorCode::BudgetExceeded)?;
            
        require!(new_spent <= institution.pac_budget_limit, ErrorCode::BudgetExceeded);
        
        // 2. Validar tiempo del deadline (debe ser futuro)
        let current_time = Clock::get()?.unix_timestamp;
        require!(deadline > current_time, ErrorCode::InvalidDeadline);
        
        // 3. Actualizar la institución
        institution.pac_budget_spent = new_spent;
        
        // 4. Inicializar la cuenta del proceso
        process.id = id;
        process.institution = institution.key();
        process.title = title;
        process.referential_budget = referential_budget;
        process.deadline = deadline;
        process.status = 0; // Convocatoria Abierta
        process.ipfs_tdr_hash = ipfs_tdr_hash;
        
        msg!("Proceso de contratacion {} publicado. Presupuesto: {}. Deadline: {}", id, referential_budget, deadline);
        Ok(())
    }

    pub fn submit_offer(ctx: Context<SubmitOffer>, process_id: u64, ipfs_proposal_hash: String) -> Result<()> {
        let process = &ctx.accounts.process;
        let offer = &mut ctx.accounts.offer;
        
        // 1. Validar que el proceso esté en estado de Convocatoria (0)
        require!(process.status == 0, ErrorCode::InvalidStatusTransition);
        
        // 2. Validar el deadline (tiempo actual <= deadline)
        let current_time = Clock::get()?.unix_timestamp;
        require!(current_time <= process.deadline, ErrorCode::OfferSubmissionClosed);
        
        // 3. Inicializar la cuenta de la oferta
        offer.provider = ctx.accounts.provider.key();
        offer.process_id = process_id;
        offer.ipfs_proposal_hash = ipfs_proposal_hash;
        offer.submission_timestamp = current_time;
        offer.expertise_verified = false; // Se verificará en la fase de calificación
        
        msg!("Oferta enviada por el proveedor {} para el proceso {}.", offer.provider, process_id);
        Ok(())
    }

    pub fn verify_offer_expertise(ctx: Context<VerifyOfferExpertise>, _process_id: u64, _provider: Pubkey, verified: bool) -> Result<()> {
        let process = &ctx.accounts.process;
        let offer = &mut ctx.accounts.offer;
        
        // 1. Validar que la institucion del proceso sea la misma que firma
        require!(process.institution == ctx.accounts.institution.key(), ErrorCode::InvalidStatusTransition);
        
        // 2. Validar que el plazo haya terminado (deadline expiro)
        let current_time = Clock::get()?.unix_timestamp;
        require!(current_time > process.deadline, ErrorCode::ProcessNotReadyForEvaluation);
        
        // 3. Validar que el proceso este en modo Convocatoria (0) o Evaluacion (1)
        require!(process.status == 0 || process.status == 1, ErrorCode::InvalidStatusTransition);
        
        // 4. Asignar verificacion
        offer.expertise_verified = verified;
        
        msg!("Evaluacion del proveedor {}: expertise_verified = {}.", offer.provider, verified);
        Ok(())
    }

    pub fn evaluate_and_award(
        ctx: Context<EvaluateAndAward>,
        process_id: u64,
        winner_provider: Pubkey,
        winning_bid_hash: String,
        final_score_hash: String,
    ) -> Result<()> {
        let process = &mut ctx.accounts.process;
        let resolution = &mut ctx.accounts.resolution;
        let winner_offer = &ctx.accounts.winner_offer;
        
        // 1. Validar que la institucion sea dueña del proceso
        require!(process.institution == ctx.accounts.institution.key(), ErrorCode::InvalidStatusTransition);
        
        // 2. Validar que el proceso este abierto para calificar (status == 0 o status == 1)
        require!(process.status == 0 || process.status == 1, ErrorCode::InvalidStatusTransition);
        
        // 3. Validar el deadline (tiempo actual > deadline)
        let current_time = Clock::get()?.unix_timestamp;
        require!(current_time > process.deadline, ErrorCode::ProcessNotReadyForEvaluation);
        
        // 4. Validar que el ganador este calificado (expertise_verified == true)
        require!(winner_offer.expertise_verified == true, ErrorCode::InvalidStatusTransition);
        
        // 5. Crear la resolucion
        resolution.process_id = process_id;
        resolution.winner_provider = winner_provider;
        resolution.winning_bid_hash = winning_bid_hash;
        resolution.final_score_hash = final_score_hash;
        resolution.timestamp = current_time;
        
        // 6. Cambiar el estado del proceso a Adjudicado (2)
        process.status = 2;
        
        msg!("Proceso {} adjudicado al proveedor {}.", process_id, winner_provider);
        Ok(())
    }
}

// ==========================================
// ESTRUCTURAS DE CUENTAS (STATE)
// ==========================================

#[account]
pub struct Institution {
    pub authority: Pubkey,
    pub name: String,
    pub pac_budget_limit: u64,
    pub pac_budget_spent: u64,
}

impl Institution {
    pub const LEN: usize = 8 + 32 + (4 + 50) + 8 + 8;
}

#[account]
pub struct ProcurementProcess {
    pub id: u64,
    pub institution: Pubkey,
    pub title: String,
    pub referential_budget: u64,
    pub deadline: i64,
    pub status: u8, // 0 = Convocatoria, 1 = Evaluacion, 2 = Adjudicado
    pub ipfs_tdr_hash: String,
}

impl ProcurementProcess {
    pub const LEN: usize = 8 + 8 + 32 + (4 + 100) + 8 + 8 + 1 + (4 + 46);
}

#[account]
pub struct OfferAccount {
    pub provider: Pubkey,
    pub process_id: u64,
    pub ipfs_proposal_hash: String,
    pub submission_timestamp: i64,
    pub expertise_verified: bool,
}

impl OfferAccount {
    pub const LEN: usize = 8 + 32 + 8 + (4 + 46) + 8 + 1;
}

#[account]
pub struct AwardResolution {
    pub process_id: u64,
    pub winner_provider: Pubkey,
    pub winning_bid_hash: String,
    pub final_score_hash: String,
    pub timestamp: i64,
}

impl AwardResolution {
    pub const LEN: usize = 8 + 8 + 32 + (4 + 46) + (4 + 46) + 8;
}

// ==========================================
// CONTEXTOS DE INSTRUCCIONES
// ==========================================

#[derive(Accounts)]
#[instruction(name: String, pac_budget_limit: u64)]
pub struct InitializeInstitution<'info> {
    #[account(
        init,
        payer = authority,
        space = Institution::LEN,
        seeds = [b"institution", authority.key().as_ref()],
        bump
    )]
    pub institution: Account<'info, Institution>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(id: u64, title: String, referential_budget: u64, deadline: i64, ipfs_tdr_hash: String)]
pub struct InitializeProcess<'info> {
    #[account(
        init,
        payer = authority,
        space = ProcurementProcess::LEN,
        seeds = [b"process", id.to_le_bytes().as_ref()],
        bump
    )]
    pub process: Account<'info, ProcurementProcess>,
    
    #[account(
        mut,
        seeds = [b"institution", authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub institution: Account<'info, Institution>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(process_id: u64, ipfs_proposal_hash: String)]
pub struct SubmitOffer<'info> {
    #[account(
        init,
        payer = provider,
        space = OfferAccount::LEN,
        seeds = [b"offer", process.key().as_ref(), provider.key().as_ref()],
        bump
    )]
    pub offer: Account<'info, OfferAccount>,
    
    #[account(
        mut,
        seeds = [b"process", process_id.to_le_bytes().as_ref()],
        bump
    )]
    pub process: Account<'info, ProcurementProcess>,
    
    #[account(mut)]
    pub provider: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(process_id: u64, provider: Pubkey, verified: bool)]
pub struct VerifyOfferExpertise<'info> {
    #[account(
        mut,
        seeds = [b"offer", process.key().as_ref(), provider.as_ref()],
        bump
    )]
    pub offer: Account<'info, OfferAccount>,
    
    #[account(
        seeds = [b"process", process_id.to_le_bytes().as_ref()],
        bump
    )]
    pub process: Account<'info, ProcurementProcess>,
    
    #[account(
        seeds = [b"institution", authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub institution: Account<'info, Institution>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(process_id: u64, winner_provider: Pubkey, winning_bid_hash: String, final_score_hash: String)]
pub struct EvaluateAndAward<'info> {
    #[account(
        init,
        payer = authority,
        space = AwardResolution::LEN,
        seeds = [b"resolution", process.key().as_ref()],
        bump
    )]
    pub resolution: Account<'info, AwardResolution>,
    
    #[account(
        mut,
        seeds = [b"process", process_id.to_le_bytes().as_ref()],
        bump
    )]
    pub process: Account<'info, ProcurementProcess>,
    
    #[account(
        seeds = [b"offer", process.key().as_ref(), winner_provider.as_ref()],
        bump
    )]
    pub winner_offer: Account<'info, OfferAccount>,
    
    #[account(
        seeds = [b"institution", authority.key().as_ref()],
        bump,
        has_one = authority
    )]
    pub institution: Account<'info, Institution>,
    
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

// ==========================================
// ERRORES PERSONALIZADOS
// ==========================================

#[error_code]
pub enum ErrorCode {
    #[msg("El presupuesto referencial excede el techo del PAC de la institucion.")]
    BudgetExceeded,
    
    #[msg("La fecha limite (deadline) debe estar en el futuro.")]
    InvalidDeadline,
    
    #[msg("La recepcion de ofertas esta cerrada para este proceso.")]
    OfferSubmissionClosed,
    
    #[msg("El plazo de recepcion aun no ha expirado; no se puede calificar.")]
    ProcessNotReadyForEvaluation,
    
    #[msg("El proceso no se encuentra en el estado adecuado para esta operacion.")]
    InvalidStatusTransition,
}
