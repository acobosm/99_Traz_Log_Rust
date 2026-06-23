#[macro_use] extern crate rocket;

use base64::Engine;
use std::sync::Arc;
use rocket::State;
use rocket::serde::json::{Json, serde_json::json, serde_json::Value};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::Header;
use rocket::{Request, Response};

// =========================================================================
// ESTRUCTURAS Y ESTADO GLOBAL
// =========================================================================

struct AppState {
    pinata_jwt: Option<String>,
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

#[options("/<_path..>")]
fn all_options(_path: std::path::PathBuf) -> &'static str {
    ""
}

// =========================================================================
// SOPORTE PARA SUBIR A IPFS (PINATA O LOCAL MOCK)
// =========================================================================

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
    let cid = format!("QmMockCID{}", uuid::Uuid::new_v4().simple().to_string());
    println!("Simulación: Archivo guardado en {:?}. CID asignado: {}", dest_path, cid);
    Ok(cid)
}

// =========================================================================
// CONTROLADORES DE RUTA (ENDPOINTS)
// =========================================================================

#[get("/")]
fn index() -> &'static str {
    "IMtBProcurement IPFS Bridge Backend (Rocket) - On-Line"
}

#[get("/api/status")]
fn get_status(state: &State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "ipfsMode": if state.pinata_jwt.is_some() { "Real IPFS (Pinata)" } else { "Simulado (Local)" }
    }))
}

#[allow(dead_code)]
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

    // 2. Subir a IPFS y devolver CID
    let ipfs_hash = match pin_file_to_ipfs(file_bytes, req.tdr_file_name.clone(), &state.pinata_jwt).await {
        Ok(hash) => hash,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::InternalServerError,
            Json(json!({ "error": format!("Error al subir el archivo TDR a IPFS: {}", e) }))
        ))
    };

    Ok(Json(json!({
        "ipfsHash": ipfs_hash
    })))
}

#[allow(dead_code)]
#[derive(rocket::serde::Deserialize)]
struct SubmitOfferRequest {
    process_id: u64,
    provider_type: String,
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

    // 2. Subir a IPFS y devolver CID
    let ipfs_hash = match pin_file_to_ipfs(file_bytes, req.proposal_file_name.clone(), &state.pinata_jwt).await {
        Ok(hash) => hash,
        Err(e) => return Err(rocket::response::status::Custom(
            rocket::http::Status::InternalServerError,
            Json(json!({ "error": format!("Error al subir la oferta a IPFS: {}", e) }))
        ))
    };

    Ok(Json(json!({
        "ipfsHash": ipfs_hash
    })))
}

// =========================================================================
// LANZADOR DEL SERVIDOR
// =========================================================================

#[launch]
fn rocket() -> _ {
    dotenvy::dotenv().ok();
    
    let pinata_jwt = std::env::var("PINATA_JWT").ok().filter(|val| !val.trim().is_empty());
    
    println!("==============================================================");
    println!("Inicializando Backend IPFS Bridge...");
    println!("IPFS Mode: {}", if pinata_jwt.is_some() { "Real IPFS (Pinata)" } else { "Simulado (Local)" });
    println!("==============================================================");
    
    let app_state = Arc::new(AppState {
        pinata_jwt,
    });

    rocket::build()
        .manage(app_state)
        .attach(CORS)
        .mount("/", routes![
            index,
            get_status,
            create_process,
            submit_offer,
            all_options
        ])
}
