#[macro_use] extern crate rocket;

use anchor_lang::{
    solana_program::instruction::Instruction as AnchorInstruction,
    prelude::Pubkey as AnchorPubkey,
    InstructionData, ToAccountMetas,
    AnchorDeserialize, Discriminator,
};
use base64::Engine;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_filter::{RpcFilterType, Memcmp};
use solana_sdk::{
    pubkey::Pubkey as SdkPubkey,
    signature::{Keypair, read_keypair_file},
    signer::Signer,
    transaction::Transaction,
    instruction::Instruction as SdkInstruction,
};
use std::sync::Arc;
use std::fs::File;
use std::io::Write;
use rocket::State;
use rocket::serde::json::{Json, serde_json::json, serde_json::Value};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};

// =========================================================================
// ESTRUCTURAS Y ESTADO GLOBAL
// =========================================================================

struct AppState {
    rpc_client: RpcClient,
    program_id: SdkPubkey,
    authority: Keypair,
    provider_a: Keypair,
    provider_b: Keypair,
    pinata_jwt: Option<String>,
}

// =========================================================================
// CONVERSORES DE TIPO (SDK <-> ANCHOR)
// =========================================================================

fn to_anchor_pubkey(sdk_pubkey: SdkPubkey) -> AnchorPubkey {
    AnchorPubkey::from(sdk_pubkey.to_bytes())
}

fn to_sdk_pubkey(anchor_pubkey: AnchorPubkey) -> SdkPubkey {
    SdkPubkey::new_from_array(anchor_pubkey.to_bytes())
}

fn to_sdk_instruction(ix: AnchorInstruction) -> SdkInstruction {
    let accounts = ix.accounts.into_iter().map(|am| {
        solana_sdk::instruction::AccountMeta {
            pubkey: SdkPubkey::new_from_array(am.pubkey.to_bytes()),
            is_signer: am.is_signer,
            is_writable: am.is_writable,
        }
    }).collect();
    
    SdkInstruction {
        program_id: SdkPubkey::new_from_array(ix.program_id.to_bytes()),
        accounts,
        data: ix.data,
    }
}

// =========================================================================
// CONFIGURACIÓN DE CORS
// =========================================================================

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new("Access-Control-Allow-Methods", "POST, GET, PATCH, OPTIONS, DELETE"));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[options("/<path..>")]
fn all_options(path: std::path::PathBuf) -> &'static str {
    ""
}

// =========================================================================
// MÉTODOS DE SOPORTE: WALLETS, AIRDROPS, IPFS
// =========================================================================

fn get_or_create_keypair(name: &str) -> Keypair {
    let path = format!("{}.json", name);
    
    // 1. Intentar cargar localmente
    if let Ok(kp) = read_keypair_file(&path) {
        println!("Cargado keypair local '{}': {}", name, kp.pubkey());
        return kp;
    }
    
    // 2. Si es authority y no está local, intentar cargar de la config global de Solana CLI
    if name == "authority" {
        if let Ok(home) = std::env::var("HOME") {
            let global_path = std::path::Path::new(&home).join(".config/solana/id.json");
            if let Ok(kp) = read_keypair_file(&global_path) {
                println!("Cargado keypair institucional global de Solana CLI: {}", kp.pubkey());
                return kp;
            }
        }
    }
    
    // 3. Generar uno nuevo y guardarlo
    let kp = Keypair::new();
    let bytes = kp.to_bytes().to_vec();
    if let Ok(json_str) = serde_json::to_string(&bytes) {
        if let Ok(mut file) = File::create(&path) {
            let _ = file.write_all(json_str.as_bytes());
        }
    }
    println!("Generado y guardado nuevo keypair para '{}': {}", name, kp.pubkey());
    kp
}

fn fund_if_needed(rpc_client: &RpcClient, pubkey: &SdkPubkey, name: &str) {
    if let Ok(balance) = rpc_client.get_balance(pubkey) {
        if balance < 1_000_000_000 { // Menos de 1 SOL
            println!("Balance de '{}' ({}) es bajo ({} SOL). Solicitando airdrop...", name, pubkey, balance as f64 / 1_000_000_000.0);
            match rpc_client.request_airdrop(pubkey, 2_000_000_000) {
                Ok(sig) => {
                    let mut retries = 10;
                    while retries > 0 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        if rpc_client.confirm_transaction(&sig).unwrap_or(false) {
                            println!("Airdrop de 2 SOL para '{}' confirmado con éxito.", name);
                            break;
                        }
                        retries -= 1;
                    }
                }
                Err(e) => {
                    println!("No se pudo solicitar airdrop para '{}': {}. Asegúrate de que el validador esté corriendo.", name, e);
                }
            }
        } else {
            println!("Balance de '{}' ({}) es suficiente: {} SOL", name, pubkey, balance as f64 / 1_000_000_000.0);
        }
    }
}

async fn pin_file_to_ipfs(file_bytes: Vec<u8>, file_name: String, jwt: &Option<String>) -> Result<String, String> {
    if let Some(token) = jwt {
        if !token.trim().is_empty() {
            println!("Subiendo '{}' a IPFS real usando Pinata...", file_name);
            let client = reqwest::Client::builder()
                .build()
                .map_err(|e| format!("Error creando cliente HTTP: {}", e))?;
                
            let form = reqwest::multipart::Form::new()
                .part("file", reqwest::multipart::Part::bytes(file_bytes).file_name(file_name));
            
            let res = client.post("https://api.pinata.cloud/pinning/pinFileToIPFS")
                .bearer_auth(token)
                .multipart(form)
                .send()
                .await
                .map_err(|e| format!("Error enviando archivo a Pinata: {}", e))?;
                
            let status = res.status();
            if status.is_success() {
                let body: Value = res.json().await
                    .map_err(|e| format!("Error decodificando respuesta de Pinata: {}", e))?;
                if let Some(cid) = body["IpfsHash"].as_str() {
                    println!("Archivo subido con éxito a IPFS. CID: {}", cid);
                    return Ok(cid.to_string());
                }
                return Err("No se encontró 'IpfsHash' en la respuesta de Pinata".to_string());
            } else {
                let err_text = res.text().await.unwrap_or_default();
                return Err(format!("Pinata rechazó la petición ({}): {}", status, err_text));
            }
        }
    }
    
    // Fallback: Modo de simulación local
    println!("Pinata JWT no configurado en .env. Usando modo de SIMULACIÓN local.");
    let mock_dir = std::path::Path::new("ipfs_mock");
    if !mock_dir.exists() {
        let _ = std::fs::create_dir_all(mock_dir);
    }
    let sanitized_name = file_name.replace("/", "_");
    let dest_path = mock_dir.join(&sanitized_name);
    let _ = std::fs::write(&dest_path, file_bytes);
    
    // Generar un hash determinista tipo CID para consistencia
    let cid = format!("QmMockCID{}", SdkPubkey::new_unique().to_string());
    println!("Simulación: Archivo guardado en {:?}. CID asignado: {}", dest_path, cid);
    Ok(cid)
}

async fn submit_tx(instructions: &[SdkInstruction], rpc_client: &RpcClient, signer: &Keypair) -> Result<String, String> {
    let blockhash = rpc_client.get_latest_blockhash()
        .map_err(|e| format!("Error obteniendo blockhash reciente: {}", e))?;
        
    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&signer.pubkey()),
        &[signer],
        blockhash
    );
    
    let signature = rpc_client.send_and_confirm_transaction(&tx)
        .map_err(|e| format!("Error enviando transacción a Solana: {}", e))?;
        
    Ok(signature.to_string())
}

// =========================================================================
// CONTROLADORES DE RUTA (ENDPOINTS)
// =========================================================================

#[get("/")]
fn index() -> &'static str {
    "IMtBProcurement API Backend (Rocket) - On-Line"
}

#[get("/api/status")]
fn get_status(state: &State<Arc<AppState>>) -> Json<Value> {
    let auth_pubkey = state.authority.pubkey();
    let prov_a_pubkey = state.provider_a.pubkey();
    let prov_b_pubkey = state.provider_b.pubkey();
    
    let auth_balance = state.rpc_client.get_balance(&auth_pubkey).unwrap_or(0);
    let prov_a_balance = state.rpc_client.get_balance(&prov_a_pubkey).unwrap_or(0);
    let prov_b_balance = state.rpc_client.get_balance(&prov_b_pubkey).unwrap_or(0);
    
    let (institution_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"institution", auth_pubkey.as_ref()],
        &to_anchor_pubkey(state.program_id),
    );
    let institution_pda_sdk = to_sdk_pubkey(institution_pda_anchor);
    
    let mut institution_data = Value::Null;
    if let Ok(acc) = state.rpc_client.get_account(&institution_pda_sdk) {
        let mut data_ref = &acc.data[8..];
        if let Ok(inst) = blockchain::Institution::deserialize(&mut data_ref) {
            institution_data = json!({
                "publicKey": institution_pda_sdk.to_string(),
                "name": inst.name,
                "pacBudgetLimit": inst.pac_budget_limit,
                "pacBudgetSpent": inst.pac_budget_spent
            });
        }
    }
    
    Json(json!({
        "programId": state.program_id.to_string(),
        "ipfsMode": if state.pinata_jwt.is_some() { "Real IPFS (Pinata)" } else { "Simulado (Local)" },
        "authority": {
            "publicKey": auth_pubkey.to_string(),
            "balance": auth_balance as f64 / 1_000_000_000.0,
            "institution": institution_data
        },
        "providerA": {
            "publicKey": prov_a_pubkey.to_string(),
            "balance": prov_a_balance as f64 / 1_000_000_000.0,
        },
        "providerB": {
            "publicKey": prov_b_pubkey.to_string(),
            "balance": prov_b_balance as f64 / 1_000_000_000.0,
        }
    }))
}

#[derive(rocket::serde::Deserialize)]
struct CreateInstitutionRequest {
    name: String,
    pac_budget_limit: u64,
}

#[post("/api/institutions", data = "<req>")]
async fn create_institution(req: Json<CreateInstitutionRequest>, state: &State<Arc<AppState>>) -> Result<Json<Value>, rocket::response::status::Custom<Json<Value>>> {
    let auth_pubkey = state.authority.pubkey();
    let program_id_anchor = to_anchor_pubkey(state.program_id);
    let (institution_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"institution", auth_pubkey.as_ref()],
        &program_id_anchor,
    );

    let inst_instruction_anchor = AnchorInstruction::new_with_bytes(
        program_id_anchor,
        &blockchain::instruction::InitializeInstitution {
            name: req.name.clone(),
            pac_budget_limit: req.pac_budget_limit,
        }.data(),
        blockchain::accounts::InitializeInstitution {
            institution: institution_pda_anchor,
            authority: to_anchor_pubkey(auth_pubkey),
            system_program: anchor_lang::solana_program::system_program::id(),
        }.to_account_metas(None),
    );

    let signature = match submit_tx(&[to_sdk_instruction(inst_instruction_anchor)], &state.rpc_client, &state.authority).await {
        Ok(sig) => sig,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": format!("Error en Solana: {}", e) }))
        ))
    };

    Ok(Json(json!({
        "success": true,
        "institutionPda": to_sdk_pubkey(institution_pda_anchor).to_string(),
        "signature": signature,
    })))
}

#[derive(rocket::serde::Deserialize)]
struct CreateProcessRequest {
    id: u64,
    title: String,
    referential_budget: u64,
    deadline_seconds_from_now: i64,
    tdr_file_base64: String,
    tdr_file_name: String,
}

#[post("/api/processes", data = "<req>")]
async fn create_process(req: Json<CreateProcessRequest>, state: &State<Arc<AppState>>) -> Result<Json<Value>, rocket::response::status::Custom<Json<Value>>> {
    // 1. Decodificar archivo TDR
    let file_bytes = match base64::engine::general_purpose::STANDARD.decode(&req.tdr_file_base64) {
        Ok(bytes) => bytes,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": format!("Base64 del TDR inválido: {}", e) }))
        ))
    };

    // 2. Subir a IPFS
    let ipfs_hash = match pin_file_to_ipfs(file_bytes, req.tdr_file_name.clone(), &state.pinata_jwt).await {
        Ok(hash) => hash,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::InternalServerError,
            Json(json!({ "error": format!("Error al subir el archivo TDR a IPFS: {}", e) }))
        ))
    };

    // 3. Calcular deadline
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let deadline = current_time + req.deadline_seconds_from_now;

    // 4. Derivar PDAs
    let auth_pubkey = state.authority.pubkey();
    let program_id_anchor = to_anchor_pubkey(state.program_id);
    let (institution_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"institution", auth_pubkey.as_ref()],
        &program_id_anchor,
    );
    let (process_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"process", req.id.to_le_bytes().as_ref()],
        &program_id_anchor,
    );

    // 5. Construir instrucción
    let proc_instruction_anchor = AnchorInstruction::new_with_bytes(
        program_id_anchor,
        &blockchain::instruction::InitializeProcess {
            id: req.id,
            title: req.title.clone(),
            referential_budget: req.referential_budget,
            deadline,
            ipfs_tdr_hash: ipfs_hash.clone(),
        }.data(),
        blockchain::accounts::InitializeProcess {
            process: process_pda_anchor,
            institution: institution_pda_anchor,
            authority: to_anchor_pubkey(auth_pubkey),
            system_program: anchor_lang::solana_program::system_program::id(),
        }.to_account_metas(None),
    );

    // 6. Firmar y enviar transacción
    let signature = match submit_tx(&[to_sdk_instruction(proc_instruction_anchor)], &state.rpc_client, &state.authority).await {
        Ok(sig) => sig,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": format!("Error de Solana al crear el proceso: {}", e) }))
        ))
    };

    Ok(Json(json!({
        "success": true,
        "processPda": to_sdk_pubkey(process_pda_anchor).to_string(),
        "ipfsHash": ipfs_hash,
        "deadline": deadline,
        "signature": signature
    })))
}

#[derive(rocket::serde::Deserialize)]
struct SubmitOfferRequest {
    process_id: u64,
    provider_type: String, // "A" o "B"
    proposal_file_base64: String,
    proposal_file_name: String,
}

#[post("/api/processes/submit_offer", data = "<req>")]
async fn submit_offer(req: Json<SubmitOfferRequest>, state: &State<Arc<AppState>>) -> Result<Json<Value>, rocket::response::status::Custom<Json<Value>>> {
    // 1. Decodificar archivo de la oferta
    let file_bytes = match base64::engine::general_purpose::STANDARD.decode(&req.proposal_file_base64) {
        Ok(bytes) => bytes,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": format!("Base64 de la oferta inválido: {}", e) }))
        ))
    };

    // 2. Subir a IPFS
    let ipfs_hash = match pin_file_to_ipfs(file_bytes, req.proposal_file_name.clone(), &state.pinata_jwt).await {
        Ok(hash) => hash,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::InternalServerError,
            Json(json!({ "error": format!("Error al subir la oferta a IPFS: {}", e) }))
        ))
    };

    // 3. Obtener keypair del proveedor solicitante
    let signer = match req.provider_type.as_str() {
        "A" => &state.provider_a,
        "B" => &state.provider_b,
        _ => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": "provider_type debe ser 'A' o 'B'" }))
        ))
    };
    let provider_pubkey = signer.pubkey();

    // 4. Derivar PDAs
    let program_id_anchor = to_anchor_pubkey(state.program_id);
    let (process_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"process", req.process_id.to_le_bytes().as_ref()],
        &program_id_anchor,
    );
    let (offer_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"offer", process_pda_anchor.as_ref(), provider_pubkey.as_ref()],
        &program_id_anchor,
    );

    // 5. Construir instrucción
    let offer_instruction_anchor = AnchorInstruction::new_with_bytes(
        program_id_anchor,
        &blockchain::instruction::SubmitOffer {
            process_id: req.process_id,
            ipfs_proposal_hash: ipfs_hash.clone(),
        }.data(),
        blockchain::accounts::SubmitOffer {
            offer: offer_pda_anchor,
            process: process_pda_anchor,
            provider: to_anchor_pubkey(provider_pubkey),
            system_program: anchor_lang::solana_program::system_program::id(),
        }.to_account_metas(None),
    );

    // 6. Firmar y enviar transacción
    let signature = match submit_tx(&[to_sdk_instruction(offer_instruction_anchor)], &state.rpc_client, signer).await {
        Ok(sig) => sig,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": format!("Error de Solana al enviar oferta: {}", e) }))
        ))
    };

    Ok(Json(json!({
        "success": true,
        "offerPda": to_sdk_pubkey(offer_pda_anchor).to_string(),
        "ipfsHash": ipfs_hash,
        "signature": signature
    })))
}

#[derive(rocket::serde::Deserialize)]
struct VerifyOfferRequest {
    process_id: u64,
    provider: String,
    verified: bool,
}

#[post("/api/processes/verify_offer", data = "<req>")]
async fn verify_offer(req: Json<VerifyOfferRequest>, state: &State<Arc<AppState>>) -> Result<Json<Value>, rocket::response::status::Custom<Json<Value>>> {
    let provider_pubkey_sdk = match req.provider.parse::<SdkPubkey>() {
        Ok(pk) => pk,
        Err(_) => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": "Provider Pubkey inválido" }))
        ))
    };
    let provider_pubkey_anchor = to_anchor_pubkey(provider_pubkey_sdk);

    let auth_pubkey = state.authority.pubkey();
    let program_id_anchor = to_anchor_pubkey(state.program_id);
    let (institution_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"institution", auth_pubkey.as_ref()],
        &program_id_anchor,
    );
    let (process_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"process", req.process_id.to_le_bytes().as_ref()],
        &program_id_anchor,
    );
    let (offer_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"offer", process_pda_anchor.as_ref(), provider_pubkey_anchor.as_ref()],
        &program_id_anchor,
    );

    let verify_instruction_anchor = AnchorInstruction::new_with_bytes(
        program_id_anchor,
        &blockchain::instruction::VerifyOfferExpertise {
            _process_id: req.process_id,
            _provider: provider_pubkey_anchor,
            verified: req.verified,
        }.data(),
        blockchain::accounts::VerifyOfferExpertise {
            offer: offer_pda_anchor,
            process: process_pda_anchor,
            institution: institution_pda_anchor,
            authority: to_anchor_pubkey(auth_pubkey),
        }.to_account_metas(None),
    );

    let signature = match submit_tx(&[to_sdk_instruction(verify_instruction_anchor)], &state.rpc_client, &state.authority).await {
        Ok(sig) => sig,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": format!("Error de Solana al calificar la oferta: {}", e) }))
        ))
    };

    Ok(Json(json!({
        "success": true,
        "signature": signature
    })))
}

#[derive(rocket::serde::Deserialize)]
struct AwardRequest {
    process_id: u64,
    winner_provider: String,
    winning_bid_hash: String,
    final_score_hash: String,
}

#[post("/api/processes/award", data = "<req>")]
async fn award_process(req: Json<AwardRequest>, state: &State<Arc<AppState>>) -> Result<Json<Value>, rocket::response::status::Custom<Json<Value>>> {
    let winner_pubkey_sdk = match req.winner_provider.parse::<SdkPubkey>() {
        Ok(pk) => pk,
        Err(_) => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": "Winner Pubkey inválido" }))
        ))
    };
    let winner_pubkey_anchor = to_anchor_pubkey(winner_pubkey_sdk);

    let auth_pubkey = state.authority.pubkey();
    let program_id_anchor = to_anchor_pubkey(state.program_id);
    let (institution_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"institution", auth_pubkey.as_ref()],
        &program_id_anchor,
    );
    let (process_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"process", req.process_id.to_le_bytes().as_ref()],
        &program_id_anchor,
    );
    let (winner_offer_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"offer", process_pda_anchor.as_ref(), winner_pubkey_anchor.as_ref()],
        &program_id_anchor,
    );
    let (resolution_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"resolution", process_pda_anchor.as_ref()],
        &program_id_anchor,
    );

    let award_instruction_anchor = AnchorInstruction::new_with_bytes(
        program_id_anchor,
        &blockchain::instruction::EvaluateAndAward {
            process_id: req.process_id,
            winner_provider: winner_pubkey_anchor,
            winning_bid_hash: req.winning_bid_hash.clone(),
            final_score_hash: req.final_score_hash.clone(),
        }.data(),
        blockchain::accounts::EvaluateAndAward {
            resolution: resolution_pda_anchor,
            process: process_pda_anchor,
            winner_offer: winner_offer_pda_anchor,
            institution: institution_pda_anchor,
            authority: to_anchor_pubkey(auth_pubkey),
            system_program: anchor_lang::solana_program::system_program::id(),
        }.to_account_metas(None),
    );

    let signature = match submit_tx(&[to_sdk_instruction(award_instruction_anchor)], &state.rpc_client, &state.authority).await {
        Ok(sig) => sig,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::BadRequest,
            Json(json!({ "error": format!("Error de Solana al adjudicar el proceso: {}", e) }))
        ))
    };

    Ok(Json(json!({
        "success": true,
        "resolutionPda": to_sdk_pubkey(resolution_pda_anchor).to_string(),
        "signature": signature
    })))
}

#[get("/api/processes")]
fn list_processes(state: &State<Arc<AppState>>) -> Json<Value> {
    let accounts = state.rpc_client.get_program_accounts(&state.program_id).unwrap_or_default();
    
    let processes: Vec<Value> = accounts.into_iter().filter_map(|(pubkey, account)| {
        if account.data.len() >= 8 && &account.data[0..8] == &blockchain::ProcurementProcess::DISCRIMINATOR[..] {
            let mut data_ref = &account.data[8..];
            if let Ok(process) = blockchain::ProcurementProcess::deserialize(&mut data_ref) {
                return Some(json!({
                    "publicKey": pubkey.to_string(),
                    "id": process.id,
                    "institution": process.institution.to_string(),
                    "title": process.title,
                    "referentialBudget": process.referential_budget,
                    "deadline": process.deadline,
                    "status": process.status,
                    "ipfsTdrHash": process.ipfs_tdr_hash,
                }));
            }
        }
        None
    }).collect();

    Json(json!(processes))
}

#[get("/api/processes/<id>")]
fn get_process(id: u64, state: &State<Arc<AppState>>) -> Result<Json<Value>, rocket::response::status::Custom<Json<Value>>> {
    let program_id_anchor = to_anchor_pubkey(state.program_id);
    let (process_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"process", id.to_le_bytes().as_ref()],
        &program_id_anchor,
    );
    let process_pda_sdk = to_sdk_pubkey(process_pda_anchor);

    let process_account = match state.rpc_client.get_account(&process_pda_sdk) {
        Ok(acc) => acc,
        Err(_) => return Err(rocket::response::status::Custom(
            rocket::http::Status::NotFound,
            Json(json!({ "error": "Proceso no encontrado" }))
        ))
    };

    let mut data_ref = &process_account.data[8..];
    let process = match blockchain::ProcurementProcess::deserialize(&mut data_ref) {
        Ok(proc) => proc,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::InternalServerError,
            Json(json!({ "error": format!("Error deserializando proceso: {}", e) }))
        ))
    };

    // 1. Obtener todas las ofertas del proceso
    let filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            40, // offset del id de proceso (8 discriminator + 32 provider)
            id.to_le_bytes().to_vec()
        ))
    ];
    let offer_accounts = state.rpc_client.get_program_accounts_with_config(
        &state.program_id,
        solana_client::rpc_config::RpcProgramAccountsConfig {
            filters: Some(filters),
            account_config: solana_client::rpc_config::RpcAccountInfoConfig {
                encoding: None,
                ..Default::default()
            },
            ..Default::default()
        }
    ).unwrap_or_default();

    let offers: Vec<Value> = offer_accounts.into_iter().filter_map(|(pubkey, account)| {
        if account.data.len() >= 8 && &account.data[0..8] == &blockchain::OfferAccount::DISCRIMINATOR[..] {
            let mut data_ref = &account.data[8..];
            if let Ok(offer) = blockchain::OfferAccount::deserialize(&mut data_ref) {
                return Some(json!({
                    "publicKey": pubkey.to_string(),
                    "provider": offer.provider.to_string(),
                    "processId": offer.process_id,
                    "ipfsProposalHash": offer.ipfs_proposal_hash,
                    "submissionTimestamp": offer.submission_timestamp,
                    "expertiseVerified": offer.expertise_verified,
                }));
            }
        }
        None
    }).collect();

    // 2. Obtener AwardResolution si existe
    let (resolution_pda_anchor, _) = AnchorPubkey::find_program_address(
        &[b"resolution", process_pda_anchor.as_ref()],
        &program_id_anchor,
    );
    let resolution_pda_sdk = to_sdk_pubkey(resolution_pda_anchor);
    let mut resolution_val = Value::Null;
    if let Ok(res_acc) = state.rpc_client.get_account(&resolution_pda_sdk) {
        let mut res_data = &res_acc.data[8..];
        if let Ok(res) = blockchain::AwardResolution::deserialize(&mut res_data) {
            resolution_val = json!({
                "publicKey": resolution_pda_sdk.to_string(),
                "winnerProvider": res.winner_provider.to_string(),
                "winningBidHash": res.winning_bid_hash,
                "finalScoreHash": res.final_score_hash,
                "timestamp": res.timestamp,
            });
        }
    }

    Ok(Json(json!({
        "publicKey": process_pda_sdk.to_string(),
        "id": process.id,
        "institution": process.institution.to_string(),
        "title": process.title,
        "referentialBudget": process.referential_budget,
        "deadline": process.deadline,
        "status": process.status,
        "ipfsTdrHash": process.ipfs_tdr_hash,
        "offers": offers,
        "resolution": resolution_val
    })))
}

// =========================================================================
// LANZADOR DEL SERVIDOR
// =========================================================================

#[launch]
fn rocket() -> _ {
    // 1. Cargar variables de entorno
    dotenvy::dotenv().ok();
    
    let rpc_url = std::env::var("SOLANA_RPC_URL").unwrap_or_else(|_| "http://localhost:8899".to_string());
    let prog_id_str = std::env::var("PROGRAM_ID").unwrap_or_else(|_| "HR3UbH45KuTanX5yiNDnPnYXr19r7mNuzUPPtj6acDJJ".to_string());
    let pinata_jwt = std::env::var("PINATA_JWT").ok().filter(|val| !val.trim().is_empty());
    
    let program_id = prog_id_str.parse::<SdkPubkey>().expect("PROGRAM_ID inválido en entorno");
    let rpc_client = RpcClient::new(rpc_url);

    println!("==============================================================");
    println!("Inicializando Backend de Compras Públicas Inmutables...");
    println!("RPC URL: {}", rpc_client.url());
    println!("Program ID: {}", program_id);
    println!("==============================================================");

    // 2. Cargar/crear keypairs del sistema
    let authority = get_or_create_keypair("authority");
    let provider_a = get_or_create_keypair("provider_a");
    let provider_b = get_or_create_keypair("provider_b");
    
    // 3. Fondeo automático de cuentas si están en localnet
    if rpc_client.url().contains("localhost") || rpc_client.url().contains("127.0.0.1") {
        println!("Fondeando cuentas para el entorno localnet...");
        fund_if_needed(&rpc_client, &authority.pubkey(), "authority");
        fund_if_needed(&rpc_client, &provider_a.pubkey(), "provider_a");
        fund_if_needed(&rpc_client, &provider_b.pubkey(), "provider_b");
    }
    
    let app_state = Arc::new(AppState {
        rpc_client,
        program_id,
        authority,
        provider_a,
        provider_b,
        pinata_jwt,
    });

    rocket::build()
        .manage(app_state)
        .attach(CORS)
        .mount("/", routes![
            index,
            get_status,
            create_institution,
            create_process,
            submit_offer,
            verify_offer,
            award_process,
            list_processes,
            get_process,
            all_options
        ])
}
