use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use eyre::Result;
use ghostink_shared::{CreatePasteRequest, CreatePasteResponse, GetPasteResponse};
use serde::Deserialize;
use tracing::{error, info, warn};

#[derive(Deserialize)]
struct Env {
    #[serde(rename = "ghostink_api_addr")]
    api_addr: String,
    #[serde(rename = "ghostink_database_url")]
    database_url: String,
}

#[derive(Clone)]
struct ApiState {
    database: sqlx::PgPool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting GhostInk API server");

    let env = envy::from_env::<Env>()?;

    info!("Connecting to database");
    let database = sqlx::PgPool::connect(&env.database_url).await?;

    info!("Running database migrations");
    sqlx::migrate!().run(&database).await?;

    let app = Router::new()
        .route("/", post(create_paste))
        .route("/{uuid}", get(get_paste))
        .route("/clean", delete(clean_expired))
        .route("/health", get(health_check))
        .with_state(ApiState { database });

    info!("Binding to address: {}", env.api_addr);
    let listener = tokio::net::TcpListener::bind(&env.api_addr).await?;

    info!("Server started successfully on {}", env.api_addr);
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create a new paste with encrypted content.
/// Content is already encrypted by the client before reaching this endpoint.
async fn create_paste(
    State(state): State<ApiState>,
    Json(payload): Json<CreatePasteRequest>,
) -> Result<(StatusCode, Json<CreatePasteResponse>), StatusCode> {
    info!("Creating new paste");
    // Calculate expiration time
    let expires_at = if let Some(expires_at_str) = payload.expires_at {
        // Parse the ISO 8601 timestamp provided by client
        chrono::DateTime::parse_from_rfc3339(&expires_at_str)
            .map_err(|e| {
                warn!(error = %e, "Failed to parse expiration timestamp");
                StatusCode::BAD_REQUEST
            })?
            .with_timezone(&chrono::Utc)
    } else {
        // Default to 1 day from now
        chrono::Utc::now() + chrono::Duration::days(1)
    };

    let uuid = sqlx::query_scalar!(
        "INSERT INTO pastes (uuid, content, expires_at) VALUES ($1, $2, $3) RETURNING uuid",
        uuid::Uuid::new_v4().to_string(),
        payload.content,
        expires_at
    )
    .fetch_one(&state.database)
    .await
    .map_err(|e| {
        error!(error = %e, "Failed to insert paste into database");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    info!(uuid = %uuid, expires_at = %expires_at, "Paste created successfully");
    Ok((StatusCode::CREATED, Json(CreatePasteResponse { uuid })))
}

/// Retrieve a paste by UUID if it hasn't expired.
/// Returns the encrypted content which must be decrypted client-side.
async fn get_paste(
    State(state): State<ApiState>,
    Path(uuid): Path<String>,
) -> Result<(StatusCode, Json<GetPasteResponse>), StatusCode> {
    info!(uuid = %uuid, "Retrieving paste");
    // Only return pastes that haven't expired
    let content = sqlx::query_scalar!(
        "SELECT content FROM pastes WHERE uuid = $1 and expires_at > NOW()",
        uuid
    )
    .fetch_one(&state.database)
    .await
    .map_err(|e| {
        warn!(uuid = %uuid, error = %e, "Paste not found or expired");
        StatusCode::NOT_FOUND
    })?;

    info!(uuid = %uuid, "Paste retrieved successfully");
    Ok((StatusCode::OK, Json(GetPasteResponse { content })))
}

/// Delete all expired pastes from the database
async fn clean_expired(State(state): State<ApiState>) -> Result<StatusCode, StatusCode> {
    info!("Starting cleanup of expired pastes");
    let result = sqlx::query!("DELETE FROM pastes WHERE expires_at <= NOW()")
        .execute(&state.database)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to clean expired pastes");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!(
        rows_affected = result.rows_affected(),
        "Cleaned expired pastes"
    );

    Ok(StatusCode::OK)
}

/// Health check endpoint
async fn health_check() -> Result<StatusCode, ()> {
    info!("Health check requested");
    Ok(StatusCode::OK)
}
