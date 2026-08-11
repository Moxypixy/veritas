use std::sync::Arc;

use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ApiError, AppState};

/// Verifies a wallet's signature for the exact server-created challenge.
///
/// Kaspa wallet message formats are integrated in Task 10. The production
/// binary deliberately installs a rejecting verifier until that work exists;
/// accepting a wallet identifier without a verified signature would make
/// wallet-bound sessions forgeable. `VERITAS_ALLOW_INSECURE_AUTH=1` enables
/// the accepting verifier only for explicit local testing.
#[async_trait]
pub trait AuthVerifier: Send + Sync {
    async fn verify(
        &self,
        wallet: &str,
        challenge: &str,
        signature: &str,
        public_key: &str,
    ) -> Result<bool, ApiError>;
}

pub struct UnconfiguredAuthVerifier;

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

pub struct InsecureAcceptingAuthVerifier;

#[async_trait]
impl AuthVerifier for InsecureAcceptingAuthVerifier {
    async fn verify(
        &self,
        _wallet: &str,
        _challenge: &str,
        signature: &str,
        public_key: &str,
    ) -> Result<bool, ApiError> {
        Ok(!signature.trim().is_empty() && !public_key.trim().is_empty())
    }
}

pub fn insecure_auth_enabled() -> bool {
    std::env::var("VERITAS_ALLOW_INSECURE_AUTH").as_deref() == Ok("1")
}

pub fn auth_verifier_from_env() -> Arc<dyn AuthVerifier> {
    if insecure_auth_enabled() {
        eprintln!(
            "WARNING: VERITAS_ALLOW_INSECURE_AUTH=1 — wallet signatures are NOT cryptographically verified. Local testing only."
        );
        Arc::new(InsecureAcceptingAuthVerifier)
    } else {
        Arc::new(UnconfiguredAuthVerifier)
    }
}

#[derive(Deserialize)]
pub(crate) struct ChallengeRequest {
    wallet: String,
}

#[derive(Serialize)]
pub(crate) struct ChallengeResponse {
    nonce: String,
    expires_at: String,
    message: String,
}

#[derive(Deserialize)]
pub(crate) struct VerifyRequest {
    wallet: String,
    nonce: String,
    signature: String,
    public_key: String,
}

#[derive(Serialize)]
pub(crate) struct VerifyResponse {
    session_token: String,
}

pub(crate) async fn challenge(
    State(state): State<AppState>,
    Json(request): Json<ChallengeRequest>,
) -> Result<(StatusCode, Json<ChallengeResponse>), ApiError> {
    if request.wallet.trim().is_empty() {
        return Err(ApiError::bad_request("wallet is required"));
    }

    let now = state.clock.now();
    let expires_at = now + Duration::minutes(5);
    let nonce = Uuid::new_v4().to_string();
    let challenge = format!(
        "Veritas login\nwallet={}\nnonce={nonce}\nexpires_at={}",
        request.wallet,
        expires_at.to_rfc3339()
    );
    state
        .database
        .create_challenge(
            &nonce,
            &request.wallet,
            &challenge,
            &expires_at.to_rfc3339(),
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(ChallengeResponse {
            nonce,
            expires_at: expires_at.to_rfc3339(),
            message: challenge,
        }),
    ))
}

pub(crate) async fn verify(
    State(state): State<AppState>,
    Json(request): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, ApiError> {
    if request.wallet.trim().is_empty()
        || request.nonce.trim().is_empty()
        || request.public_key.trim().is_empty()
    {
        return Err(ApiError::bad_request(
            "wallet, nonce, and public_key are required",
        ));
    }

    let now = state.clock.now();
    let challenge = state
        .database
        .challenge_for_verification(&request.nonce, &request.wallet, &now.to_rfc3339())
        .await?
        .ok_or_else(|| ApiError::unauthorized("challenge is invalid or expired"))?;
    if !state
        .auth_verifier
        .verify(
            &request.wallet,
            &challenge,
            &request.signature,
            &request.public_key,
        )
        .await?
    {
        return Err(ApiError::unauthorized("wallet signature is invalid"));
    }
    if !state
        .database
        .consume_challenge(&request.nonce, &request.wallet, &now.to_rfc3339())
        .await?
    {
        return Err(ApiError::unauthorized("challenge is invalid or expired"));
    }

    let session_token = Uuid::new_v4().to_string();
    let expires_at = now + Duration::hours(24);
    state
        .database
        .create_session(&session_token, &request.wallet, &expires_at.to_rfc3339())
        .await?;
    Ok(Json(VerifyResponse { session_token }))
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        sync::{Mutex, OnceLock},
    };

    use super::{AuthVerifier, InsecureAcceptingAuthVerifier, insecure_auth_enabled};

    static INSECURE_AUTH_ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    struct InsecureAuthEnvRestore(Option<OsString>);

    impl Drop for InsecureAuthEnvRestore {
        fn drop(&mut self) {
            // Tests hold INSECURE_AUTH_ENV_LOCK while mutating this process-global value.
            unsafe {
                match self.0.as_ref() {
                    Some(value) => std::env::set_var("VERITAS_ALLOW_INSECURE_AUTH", value),
                    None => std::env::remove_var("VERITAS_ALLOW_INSECURE_AUTH"),
                }
            }
        }
    }

    fn insecure_auth_enabled_with_env(value: Option<&str>) -> bool {
        let _lock = INSECURE_AUTH_ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("insecure auth environment lock must not be poisoned");
        let _restore = InsecureAuthEnvRestore(std::env::var_os("VERITAS_ALLOW_INSECURE_AUTH"));
        unsafe {
            match value {
                Some(value) => std::env::set_var("VERITAS_ALLOW_INSECURE_AUTH", value),
                None => std::env::remove_var("VERITAS_ALLOW_INSECURE_AUTH"),
            }
        }
        insecure_auth_enabled()
    }

    #[test]
    fn insecure_auth_environment_is_explicitly_opt_in() {
        for value in [None, Some("0"), Some("true")] {
            assert!(
                !insecure_auth_enabled_with_env(value),
                "{value:?} must not enable insecure authentication"
            );
        }
        assert!(insecure_auth_enabled_with_env(Some("1")));
    }

    #[tokio::test]
    async fn insecure_verifier_accepts_non_empty_signature_and_public_key() {
        let verifier = InsecureAcceptingAuthVerifier;
        assert!(
            verifier
                .verify("kaspatest:qq", "challenge", "sig", "pubkey")
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn insecure_verifier_rejects_empty_signature_or_public_key() {
        let verifier = InsecureAcceptingAuthVerifier;
        assert!(
            !verifier
                .verify("kaspatest:qq", "challenge", "  ", "pubkey")
                .await
                .unwrap()
        );
        assert!(
            !verifier
                .verify("kaspatest:qq", "challenge", "sig", "")
                .await
                .unwrap()
        );
    }
}
