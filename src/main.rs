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
use ed25519_dalek::{SecretKey, PublicKey, Keypair as Ed25519Keypair};

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

#[derive(Deserialize)]
struct MintTokenRequest {
    mint: String,
    destination: String,
    authority: String,
    amount: u64,
}

#[derive(Deserialize)]
struct SignMessageRequest {
    message: String,
    secret: String,
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

async fn mint_token(
    Json(req): Json<MintTokenRequest>
) -> impl IntoResponse {
    // parse the pubkeys
    let mint = Pubkey::from_str(&req.mint).unwrap();
    let destination = Pubkey::from_str(&req.destination).unwrap();
    let authority = Pubkey::from_str(&req.authority).unwrap();

    // build the mint_to instruction
    let ix = token_instruction::mint_to(
        &spl_token::id(),
        &mint,
        &destination,
        &authority,
        &[],
        req.amount,
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

async fn sign_message(
    Json(req): Json<SignMessageRequest>
) -> impl IntoResponse {
    // Check for missing fields
    if req.message.is_empty() || req.secret.is_empty() {
        return Json(json!({
            "success": false,
            "error": "Missing required fields"
        }));
    }

    // Convert base58 secret to bytes
    let secret_bytes = bs58::decode(&req.secret).into_vec().unwrap();
    
    // Create signing key from secret
    let secret_key = SecretKey::from_bytes(&secret_bytes).unwrap();
    let public_key = PublicKey::from(&secret_key);
    
    // Create keypair for signing
    let keypair = Ed25519Keypair {
        secret: secret_key,
        public: public_key,
    };
    
    // Sign the message
    let signature = keypair.sign(req.message.as_bytes());

    Json(json!({
        "success": true,
        "data": {
            "signature": BASE64.encode(signature.to_bytes()),
            "public_key": bs58::encode(public_key.to_bytes()).into_string(),
            "message": req.message
        }
    }))
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/keypair", post(generate_keypair))
        .route("//keypair", post(generate_keypair))
        .route("/token/create", post(create_token))
        .route("//token/create", post(create_token))
        .route("/token/mint", post(mint_token))
        .route("/message/sign", post(sign_message));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on {}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .await
        .unwrap();
}
