use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json};
use solana_program::pubkey::Pubkey;
use solana_sdk::{
    signature::{Keypair, Signer},
};
use spl_token::instruction as token_instruction;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::TcpListener;
use tracing::{info, error, debug};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};


#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Solana error: {0}")]
    SolanaError(String),
    #[error("Internal server error")]
    InternalError,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            AppError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::SolanaError(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalError => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string()),
        };

        let body = Json(json!({
            "success": false,
            "error": error_message
        }));

        (status, body).into_response()
    }
}
#[derive(Debug, Deserialize)]
pub struct TokenCreateRequest {
    pub mint_authority: String,
    pub mint: String,
    pub decimals: u8,
}

#[derive(Debug, Serialize)]
pub struct StandardResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KeypairResponse {
    pub pubkey: String,
    pub secret: String,
}

#[derive(Debug, Serialize)]
pub struct TokenInstructionResponse {
    pub program_id: String,
    pub accounts: Vec<AccountMetaResponse>,
    pub instruction_data: String,
}

#[derive(Debug, Serialize)]
pub struct AccountMetaResponse {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

fn base58_to_pubkey(base58_str: &str) -> Result<Pubkey, AppError> {
    Pubkey::from_str(base58_str)
        .map_err(|e| AppError::InvalidInput(format!("Invalid public key: {}", e)))
}

async fn home() -> Html<&'static str> {
    info!("Home endpoint accessed");
    Html("<h1>Solana Fellowship Backend API</h1><p>Server is running!</p>")
}

async fn generate_keypair() -> Result<Json<StandardResponse<KeypairResponse>>, AppError> {
    info!("Generating new keypair");
    
    // Generate a new Solana keypair
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey().to_string();
    let secret = bs58::encode(keypair.to_bytes()).into_string();
    
    debug!("Generated keypair with pubkey: {}", pubkey);
    
    Ok(Json(StandardResponse {
        success: true,
        data: Some(KeypairResponse {
            pubkey,
            secret,
        }),
        error: None,
    }))
}

async fn create_token(
    Json(payload): Json<TokenCreateRequest>,
) -> Result<Json<StandardResponse<TokenInstructionResponse>>, AppError> {
    info!("Creating SPL token with mint: {}", payload.mint);
    
    // Parse base58 strings into Pubkey
    let mint_authority = base58_to_pubkey(&payload.mint_authority)?;
    let mint = base58_to_pubkey(&payload.mint)?;
    
    // Create initialize mint instruction
    let instruction = token_instruction::initialize_mint(
        &spl_token::id(),
        &mint,
        &mint_authority,
        Some(&mint_authority),
        payload.decimals,
    ).map_err(|e| AppError::SolanaError(format!("Failed to create instruction: {:?}", e)))?;
    
    // Convert accounts to response format - ensure it's an array
    let accounts: Vec<AccountMetaResponse> = instruction.accounts.iter().map(|acc| {
        AccountMetaResponse {
            pubkey: acc.pubkey.to_string(),
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        }
    }).collect();
    
    debug!("Created token instruction with {} accounts", accounts.len());
    
    Ok(Json(StandardResponse {
        success: true,
        data: Some(TokenInstructionResponse {
            program_id: spl_token::id().to_string(),
            accounts,
            instruction_data: BASE64.encode(&instruction.data),
        }),
        error: None,
    }))
}


#[tokio::main]
async fn main() {
    // Initialize tracing for logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting Solana Fellowship Backend Server");

    // Build our application with a route tree
    let app = Router::new()
        .route("/", get(home))
        .route("/keypair", post(generate_keypair))
        .route("/token/create", post(create_token));

    // Run it with hyper on localhost:8080
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("🚀 Server starting on {}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    info!("✅ Server listening on {}", addr);
    
    axum::serve(listener, app)
        .await
        .unwrap();
}
