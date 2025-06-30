// src/main.rs

use axum::{
    routing::{post},
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize};
use serde_json::json;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_program::system_instruction;
use spl_token::instruction as token_instruction;
use std::net::SocketAddr;
use std::str::FromStr;
use tokio::net::TcpListener;
use bs58;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

async fn generate_keypair() -> Json<serde_json::Value> {
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey().to_string();
    let secret = bs58::encode(keypair.to_bytes()).into_string();

    Json(json!({
        "success": true,
        "data": {
            "pubkey": pubkey,
            "secret": secret
        }
    }))
}

#[derive(Deserialize)]
struct CreateTokenRequest {
    #[serde(rename = "mintAuthority")]
    mint_authority: String,
    mint: String,
    decimals: u8,
}

#[derive(Deserialize)]
struct SendSolRequest {
    from: String,
    to: String,
    lamports: u64,
}

async fn create_token(
    Json(req): Json<CreateTokenRequest>
) -> impl IntoResponse {
    let mint_authority = match Pubkey::from_str(&req.mint_authority) {
        Ok(pk) => pk,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": format!("Invalid mint_authority: {}", e)
                }))
            )
        }
    };

    let mint = match Pubkey::from_str(&req.mint) {
        Ok(pk) => pk,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": format!("Invalid mint: {}", e)
                }))
            )
        }
    };
    let ix = match token_instruction::initialize_mint(
        &spl_token::id(),
        &mint,
        &mint_authority,
        Some(&mint_authority),
        req.decimals,
    ) {
        Ok(ix) => ix,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "success": false,
                    "error": format!("Failed to build instruction: {}", e)
                }))
            )
        }
    };

    let accounts: Vec<_> = ix.accounts.iter().map(|meta| {
        json!({
            "pubkey": meta.pubkey.to_string(),
            "is_signer": meta.is_signer,
            "is_writable": meta.is_writable,
        })
    }).collect();
    let instruction_data = BASE64.encode(&ix.data);

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "program_id": ix.program_id.to_string(),
                "accounts": accounts,
                "instruction_data": instruction_data
            }
        }))
    )
}

async fn create_send_sol(
    Json(req): Json<SendSolRequest>
) -> impl IntoResponse {
    let from = match Pubkey::from_str(&req.from) {
        Ok(pk) => pk,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": format!("Invalid from address: {}", e)
                }))
            )
        }
    };

    let to = match Pubkey::from_str(&req.to) {
        Ok(pk) => pk,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": format!("Invalid to address: {}", e)
                }))
            )
        }
    };
        
    let ix = system_instruction::transfer(&from, &to, req.lamports);

    let accounts: Vec<_> = ix.accounts.iter().map(|meta| {
        meta.pubkey.to_string()
    }).collect();

    let instruction_data = BASE64.encode(&ix.data);

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "program_id": ix.program_id.to_string(),
                "accounts": accounts,
                "instruction_data": instruction_data
            }
        }))
    )
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/keypair", post(generate_keypair))
        .route("//keypair", post(generate_keypair))
        .route("/token/create", post(create_token))
        .route("//token/create", post(create_token))
        .route("/send/sol", post(create_send_sol))
        .route("//send/sol", post(create_send_sol));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on {}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .await
        .unwrap();
}
