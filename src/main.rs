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
    mintAuthority: String,
    mint: String,
    decimals: u8,
}

async fn create_token(
    Json(req): Json<CreateTokenRequest>
) -> impl IntoResponse {
    // parse the two pubkeys
    let mint_authority = Pubkey::from_str(&req.mintAuthority).unwrap();
    let mint = Pubkey::from_str(&req.mint).unwrap();

    // build the initialize_mint instruction
    let ix = token_instruction::initialize_mint(
        &spl_token::id(),
        &mint,
        &mint_authority,
        Some(&mint_authority),
        req.decimals,
    ).unwrap();

    // map account metas into JSON
    let accounts: Vec<_> = ix.accounts.iter().map(|meta| {
        json!({
            "pubkey": meta.pubkey.to_string(),
            "is_signer": meta.is_signer,
            "is_writable": meta.is_writable
        })
    }).collect();

    let instruction_data = BASE64.encode(&ix.data);

    Json(json!({
        "success": true,
        "data": {
            "program_id": ix.program_id.to_string(),
            "accounts": accounts,
            "instruction_data": instruction_data
        }
    }))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/keypair", post(generate_keypair))
        .route("//keypair", post(generate_keypair))
        .route("/token/create", post(create_token))
        .route("//token/create", post(create_token));


    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on {}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .await
        .unwrap();
}
