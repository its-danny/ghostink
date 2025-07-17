use serde::{Deserialize, Serialize};

/// Request to create a new paste
#[derive(Serialize, Deserialize)]
pub struct CreatePasteRequest {
    /// Base64-encoded encrypted content (includes nonce + ciphertext)
    pub content: String,
    /// Optional expiration time in ISO 8601 format (e.g., "2025-07-24T17:41:50+00:00")
    /// If not provided, the server will use a default expiration of 1 day
    pub expires_at: Option<String>,
}

/// Response when creating a paste
#[derive(Serialize, Deserialize)]
pub struct CreatePasteResponse {
    /// UUID that can be used to retrieve the paste
    pub uuid: String,
}

/// Response when retrieving a paste
#[derive(Serialize, Deserialize)]
pub struct GetPasteResponse {
    /// Base64-encoded encrypted content (includes nonce + ciphertext)
    pub content: String,
}
