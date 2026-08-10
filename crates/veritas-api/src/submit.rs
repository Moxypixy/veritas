use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use veritas::{VoteAnswer, commitment_hash};

use crate::{ApiError, AppState, db::NewVote};

#[derive(Deserialize)]
pub(crate) struct SubmitVoteRequest {
    region_id: String,
    necessities_pct: u8,
    employed: bool,
    duration_months: u32,
    month_key: String,
    salt: String,
    commitment: String,
    // Preserved for the future Kaspa verifier integration without exposing it
    // in aggregate responses.
    #[allow(dead_code)]
    tx_id: String,
}

#[derive(Serialize)]
pub(crate) struct SubmitVoteResponse {
    accepted: bool,
}

#[derive(Serialize)]
struct StoredAnswer<'a> {
    region_id: &'a str,
    necessities_pct: u8,
    employed: bool,
    duration_months: u32,
    month_key: &'a str,
    salt: String,
}

pub(crate) async fn submit(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SubmitVoteRequest>,
) -> Result<(StatusCode, Json<SubmitVoteResponse>), ApiError> {
    let token = bearer_token(&headers)?;
    let now = state.clock.now();
    let wallet = state
        .database
        .wallet_for_session(&token, &now.to_rfc3339())
        .await?
        .ok_or_else(|| ApiError::unauthorized("session is invalid or expired"))?;

    let observed_month = format!("{:04}-{:02}", now.year(), now.month());
    // This is intentionally an API rule: local Argent has no UTC clock and
    // cannot enforce that the caller's vote month is the current calendar one.
    if request.month_key != observed_month {
        return Err(ApiError::unprocessable(
            "month_key must match the API's observed UTC month",
        ));
    }
    let salt = parse_salt(&request.salt)?;
    let declared_commitment = parse_commitment(&request.commitment)?;
    let answer = VoteAnswer {
        region_id: request.region_id,
        necessities_pct: request.necessities_pct,
        employed: request.employed,
        duration_months: request.duration_months,
        month_key: request.month_key,
        wallet: wallet.clone(),
        salt,
    };
    let calculated_commitment =
        commitment_hash(&answer).map_err(|_| ApiError::unprocessable("invalid vote answer"))?;
    if calculated_commitment != declared_commitment {
        return Err(ApiError::unprocessable(
            "declared commitment does not match answer",
        ));
    }
    let chain_commitment = state
        .chain_verifier
        .commitment_for_vote(&wallet, &answer.month_key)
        .await?
        .ok_or_else(|| ApiError::unprocessable("no verified on-chain commitment exists"))?;
    if chain_commitment != calculated_commitment {
        return Err(ApiError::unprocessable(
            "on-chain commitment does not match answer",
        ));
    }

    // TODO(task-6): replace this transitional JSON payload with AES-256-GCM.
    // It stays in the ciphertext-shaped schema so the encryption migration does
    // not alter the personal-answer table. No Task 5 endpoint returns it.
    let ciphertext = serde_json::to_vec(&StoredAnswer {
        region_id: &answer.region_id,
        necessities_pct: answer.necessities_pct,
        employed: answer.employed,
        duration_months: answer.duration_months,
        month_key: &answer.month_key,
        salt: request.salt,
    })
    .map_err(|_| ApiError::unavailable("could not serialize vote"))?;
    let employment = if answer.employed {
        "employed"
    } else {
        "unemployed"
    };
    let created_at = now.to_rfc3339();
    state
        .database
        .store_vote(NewVote {
            wallet: &wallet,
            month_key: &answer.month_key,
            region_id: &answer.region_id,
            ciphertext: &ciphertext,
            commitment: &calculated_commitment,
            necessities_pct: answer.necessities_pct,
            employment,
            created_at: &created_at,
        })
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(SubmitVoteResponse { accepted: true }),
    ))
}

fn bearer_token(headers: &HeaderMap) -> Result<String, ApiError> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("missing bearer session"))?;
    header
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| ApiError::unauthorized("missing bearer session"))
}

fn parse_salt(value: &str) -> Result<[u8; 16], ApiError> {
    let bytes = hex::decode(value).map_err(|_| ApiError::bad_request("salt must be hex"))?;
    bytes
        .try_into()
        .map_err(|_| ApiError::bad_request("salt must be 16 bytes"))
}

fn parse_commitment(value: &str) -> Result<[u8; 32], ApiError> {
    let bytes = hex::decode(value).map_err(|_| ApiError::bad_request("commitment must be hex"))?;
    bytes
        .try_into()
        .map_err(|_| ApiError::bad_request("commitment must be 32 bytes"))
}
