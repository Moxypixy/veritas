use std::{env, sync::Arc};

use async_trait::async_trait;
use veritas_api::{
    ApiError, AppState, AuthVerifier, ChainVerifier, DataKey, Database, SystemClock, router,
};

/// The real Kaspa wallet message verifier is intentionally deferred to Task 10.
/// Starting this binary before then must not turn a supplied wallet string into
/// an authenticated identity.
struct UnconfiguredAuthVerifier;

#[async_trait]
impl AuthVerifier for UnconfiguredAuthVerifier {
    async fn verify(
        &self,
        _wallet: &str,
        _challenge: &str,
        _signature: &str,
        _public_key: &str,
    ) -> Result<bool, ApiError> {
        Err(ApiError::unavailable(
            "wallet authentication is not configured",
        ))
    }
}

/// The local Argent runtime provides no live chain query. Task 10 replaces
/// this with a verified testnet source before vote submission is enabled.
struct UnconfiguredChainVerifier;

#[async_trait]
impl ChainVerifier for UnconfiguredChainVerifier {
    async fn commitment_for_vote(
        &self,
        _wallet: &str,
        _month_key: &str,
    ) -> Result<Option<[u8; 32]>, ApiError> {
        Err(ApiError::unavailable(
            "chain commitment verification is not configured",
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if env::var("VERITAS_KASPA_NETWORK").as_deref() != Ok("kaspa_testnet_10") {
        return Err(
            "VERITAS_KASPA_NETWORK must be kaspa_testnet_10; mainnet is unsupported".into(),
        );
    }
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL must name the SQLite database, e.g. sqlite://veritas.db")?;
    let encoded_data_key =
        env::var("VERITAS_DATA_KEY").map_err(|_| "VERITAS_DATA_KEY must be set outside tests")?;
    let data_key = DataKey::from_base64(&encoded_data_key)
        .map_err(|_| "VERITAS_DATA_KEY must be a base64-encoded 32-byte key")?;
    let database = Database::connect(&database_url)
        .await
        .map_err(|error| std::io::Error::other(format!("{error:?}")))?;
    let app = router(AppState::new(
        database,
        data_key,
        Arc::new(UnconfiguredChainVerifier),
        Arc::new(UnconfiguredAuthVerifier),
        Arc::new(SystemClock),
    ));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
