//! HTTP API for the local Veritas indexer.
//!
//! The Argent covenant is currently local-runtime only. Therefore this API is
//! the trusted gate for two rules the covenant cannot prove today: a wallet
//! can submit only one ballot globally for a calendar month, and a submitted
//! `month_key` must equal the API's observed UTC month. A future chain
//! verifier must verify the commitment before this API stores an answer.

mod aggregates;
mod auth;
mod crypto;
mod db;
mod gdpr;
mod submit;

use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    Router,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};

pub use auth::AuthVerifier;
pub use crypto::DataKey;
pub use db::Database;

#[derive(Debug, Clone)]
pub struct ApiError {
    status: axum::http::StatusCode,
    message: &'static str,
}

impl ApiError {
    pub fn bad_request(message: &'static str) -> Self {
        Self {
            status: axum::http::StatusCode::BAD_REQUEST,
            message,
        }
    }

    pub fn unauthorized(message: &'static str) -> Self {
        Self {
            status: axum::http::StatusCode::UNAUTHORIZED,
            message,
        }
    }

    pub fn conflict(message: &'static str) -> Self {
        Self {
            status: axum::http::StatusCode::CONFLICT,
            message,
        }
    }

    pub fn unprocessable(message: &'static str) -> Self {
        Self {
            status: axum::http::StatusCode::UNPROCESSABLE_ENTITY,
            message,
        }
    }

    pub fn unavailable(message: &'static str) -> Self {
        Self {
            status: axum::http::StatusCode::SERVICE_UNAVAILABLE,
            message,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            axum::Json(serde_json::json!({ "error": self.message })),
        )
            .into_response()
    }
}

/// Abstracts the chain-facing check until a Kaspa-backed verifier is added.
#[async_trait]
pub trait ChainVerifier: Send + Sync {
    async fn commitment_for_vote(
        &self,
        wallet: &str,
        month_key: &str,
    ) -> Result<Option<[u8; 32]>, ApiError>;
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

#[derive(Clone)]
pub struct FixedClock(DateTime<Utc>);

impl FixedClock {
    pub fn new(now: DateTime<Utc>) -> Self {
        Self(now)
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

#[derive(Clone)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) database: Database,
    pub(crate) data_key: Arc<DataKey>,
    pub(crate) chain_verifier: Arc<dyn ChainVerifier>,
    pub(crate) auth_verifier: Arc<dyn AuthVerifier>,
    pub(crate) clock: Arc<dyn Clock>,
}

impl AppState {
    pub fn new(
        database: Database,
        data_key: DataKey,
        chain_verifier: Arc<dyn ChainVerifier>,
        auth_verifier: Arc<dyn AuthVerifier>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            database,
            data_key: Arc::new(data_key),
            chain_verifier,
            auth_verifier,
            clock,
        }
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/auth/challenge", axum::routing::post(auth::challenge))
        .route("/v1/auth/verify", axum::routing::post(auth::verify))
        .route("/v1/votes", axum::routing::post(submit::submit))
        .route("/v1/aggregates", axum::routing::get(aggregates::get))
        .route("/v1/me/export", axum::routing::get(gdpr::export))
        .route("/v1/me/answers", axum::routing::delete(gdpr::erase))
        .with_state(state)
}
