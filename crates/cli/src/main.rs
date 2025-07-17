use std::time::{SystemTime, UNIX_EPOCH};

use aes_gcm::{KeyInit, aead::Aead};
use base64::{Engine, prelude::BASE64_STANDARD};
use clap::{Parser, Subcommand};
use clap_stdin::FileOrStdin;
use eyre::Result;
use ghostink_shared::{CreatePasteRequest, CreatePasteResponse, GetPasteResponse};
use rand::RngCore;

#[derive(Parser)]
#[command(name = "ghostink")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new paste by encrypting and uploading content
    Create {
        file: FileOrStdin<String>,
        /// When the paste should expire (e.g., "1h", "2d", "1w", "30m")
        #[arg(long)]
        expires: Option<String>,
    },
    /// Get and decrypt a paste by UUID and key
    Get {
        /// UUID and key in format "uuid#key"
        uuid_key: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Commands::Create { file, expires } => {
            create_paste(file, expires).await?;
        }
        Commands::Get { uuid_key } => {
            get_paste(uuid_key).await?;
        }
    }

    Ok(())
}

async fn create_paste(file: FileOrStdin<String>, expires: Option<String>) -> Result<()> {
    let content = file.contents()?;

    // Generate a random 32-byte key for AES-256-GCM encryption
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);

    // Generate a random 12-byte nonce (number used once) for GCM mode
    let mut nonce = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);

    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key)
        .map_err(|e| eyre::eyre!("Invalid key length: {}", e))?;
    let ciphertext = cipher
        .encrypt(&nonce.into(), content.as_bytes())
        .map_err(|e| eyre::eyre!("Encryption failed: {}", e))?;

    // Prepend nonce to ciphertext before base64 encoding
    // Format: [nonce(12 bytes)][ciphertext(variable)]
    let mut nonce_and_ciphertext = Vec::with_capacity(12 + ciphertext.len());
    nonce_and_ciphertext.extend_from_slice(&nonce);
    nonce_and_ciphertext.extend_from_slice(&ciphertext);

    // Calculate expiration time if provided
    let expires_at = if let Some(expires_str) = expires {
        // Parse human-readable duration (e.g., "1h", "2d", "1w")
        let duration = humantime::parse_duration(&expires_str)
            .map_err(|e| eyre::eyre!("Invalid duration format '{}': {}", expires_str, e))?;

        let expiry_time = SystemTime::now() + duration;
        let timestamp = expiry_time
            .duration_since(UNIX_EPOCH)
            .map_err(|e| eyre::eyre!("Time calculation error: {}", e))?;

        // Convert to ISO 8601 format for the API
        Some(
            chrono::DateTime::from_timestamp(timestamp.as_secs() as i64, timestamp.subsec_nanos())
                .ok_or_else(|| eyre::eyre!("Invalid timestamp"))?
                .to_rfc3339(),
        )
    } else {
        None // API will use default (1 day)
    };

    let request = CreatePasteRequest {
        content: BASE64_STANDARD.encode(&nonce_and_ciphertext),
        expires_at,
    };

    let client = reqwest::Client::new();
    let resp = client
        .post("http://localhost:3000")
        .json(&request)
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(eyre::eyre!("Failed to create paste: {}", resp.status()));
    }

    let create_response: CreatePasteResponse = resp.json().await?;
    // Output the command to retrieve and decrypt the paste
    let key_hex = hex::encode(key);
    println!("ghostink get {}#{}", create_response.uuid, key_hex);

    Ok(())
}

async fn get_paste(uuid_key: String) -> Result<()> {
    // Parse UUID and key from "uuid#key" format
    let parts: Vec<&str> = uuid_key.split('#').collect();
    if parts.len() != 2 {
        return Err(eyre::eyre!("Invalid format. Expected 'uuid#key'"));
    }

    let uuid = parts[0];
    let key_str = parts[1];

    // Decode the hex-encoded encryption key
    let key = hex::decode(key_str).map_err(|e| eyre::eyre!("Invalid hex key: {}", e))?;

    if key.len() != 32 {
        return Err(eyre::eyre!(
            "Key must be 32 bytes long (got {} bytes)",
            key.len()
        ));
    }

    // Fetch from API
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://localhost:3000/{uuid}"))
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(eyre::eyre!("Failed to fetch paste: {}", resp.status()));
    }

    let paste_response: GetPasteResponse = resp.json().await?;

    // Decode the base64 content (nonce + ciphertext)
    let nonce_and_ciphertext = BASE64_STANDARD
        .decode(&paste_response.content)
        .map_err(|e| eyre::eyre!("Invalid base64 content: {}", e))?;

    if nonce_and_ciphertext.len() < 12 {
        return Err(eyre::eyre!(
            "Content too short to contain nonce (need at least 12 bytes)"
        ));
    }

    // Extract nonce (first 12 bytes) and ciphertext (remaining bytes)
    let nonce: [u8; 12] = nonce_and_ciphertext[0..12]
        .try_into()
        .map_err(|e| eyre::eyre!("Failed to extract nonce: {}", e))?;
    let ciphertext = &nonce_and_ciphertext[12..];

    // Decrypt the content using AES-256-GCM
    let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key)
        .map_err(|e| eyre::eyre!("Invalid key length: {}", e))?;

    let plaintext = cipher
        .decrypt(&nonce.into(), ciphertext)
        .map_err(|e| eyre::eyre!("Decryption failed: {}", e))?;

    // Convert decrypted bytes to string and output
    let content = String::from_utf8(plaintext).map_err(|e| eyre::eyre!("Invalid UTF-8: {}", e))?;

    print!("{content}");

    Ok(())
}
