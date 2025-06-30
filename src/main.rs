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
use ed25519_dalek::{SecretKey, PublicKey, Keypair as Ed25519Keypair, Signer as Ed25519Signer};



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

#[derive(Deserialize)]
struct SendTokenRequest {
    destination: String,
    mint: String,
    owner: String,
    amount: u64,
}

#[derive(Deserialize)]
struct SignMessageRequest {
    message: String,
    secret: String,
}



async fn generate_new_keypair() -> Json<serde_json::Value> {
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

async fn initialize_token_mint(
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

async fn transfer_spl_tokens(
    Json(req): Json<SendTokenRequest>
) -> impl IntoResponse {
    let destination = Pubkey::from_str(&req.destination).unwrap_or_else(|_| Pubkey::default());
    let mint = Pubkey::from_str(&req.mint).unwrap_or_else(|_| Pubkey::default());
    let owner = Pubkey::from_str(&req.owner).unwrap_or_else(|_| Pubkey::default());

    let ix = token_instruction::transfer(
        &spl_token::id(),
        &destination,
        &destination,
        &owner,
        &[],
        req.amount,
    ).unwrap_or_else(|_| {
        token_instruction::transfer(
            &spl_token::id(),
            &Pubkey::default(),
            &Pubkey::default(),
            &Pubkey::default(),
            &[],
            0,
        ).unwrap()
    });

    let accounts: Vec<_> = ix.accounts.iter().map(|meta| {
        json!({
            "pubkey": meta.pubkey.to_string(),
            "isSigner": meta.is_signer,
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


async fn transfer_sol(
    Json(req): Json<SendSolRequest>
) -> impl IntoResponse {
    let from = Pubkey::from_str(&req.from).unwrap_or_else(|_| {
        Pubkey::default()
    });

    let to = Pubkey::from_str(&req.to).unwrap_or_else(|_| {
        Pubkey::default()
    });
        
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


async fn sign_message_with_ed25519(
    Json(req): Json<SignMessageRequest>
) -> impl IntoResponse {        
    if req.message.is_empty() || req.secret.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "success": false,
                "error": "Missing required fields"
            }))
        );
    }

    let secret_bytes = match bs58::decode(&req.secret).into_vec() {
        Ok(bytes) => bytes,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid base58 secret key"
                }))
            );
        }
    };
    
    let secret_key = match SecretKey::from_bytes(&secret_bytes) {
        Ok(key) => key,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "error": "Invalid secret key"
                }))
            );
        }
    };
    
    let public_key = PublicKey::from(&secret_key);
    
    let keypair = Ed25519Keypair {
        secret: secret_key,
        public: public_key,
    };
    
    let signature = keypair.sign(req.message.as_bytes());

    (
        StatusCode::OK,
        Json(json!({
            "success": true,
            "data": {
                "signature": BASE64.encode(signature.to_bytes()),
                "public_key": bs58::encode(public_key.to_bytes()).into_string(),
                "message": req.message
            }
        }))
    )
}


#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/keypair", post(generate_new_keypair))
        .route("//keypair", post(generate_new_keypair))
        .route("/token/create", post(initialize_token_mint))
        .route("//token/create", post(initialize_token_mint))
        .route("/send/sol", post(transfer_sol))
        .route("//send/sol", post(transfer_sol))
        .route("/send/token", post(transfer_spl_tokens))
        .route("//send/token", post(transfer_spl_tokens))
        .route("/message/sign", post(sign_message_with_ed25519))
        .route("//message/sign", post(sign_message_with_ed25519));

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    println!("Listening on {}", addr);
    
    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .await
        .unwrap();
}