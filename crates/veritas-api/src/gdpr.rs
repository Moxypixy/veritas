use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, AppState, submit::bearer_token};

#[derive(Deserialize, Serialize)]
pub(crate) struct StoredAnswer {
    pub(crate) region_id: String,
    pub(crate) necessities_pct: u8,
    pub(crate) employed: bool,
    pub(crate) duration_months: u32,
    pub(crate) month_key: String,
    pub(crate) salt: String,
}

#[derive(Serialize)]
pub(crate) struct ExportResponse {
    answers: Vec<StoredAnswer>,
}

pub(crate) async fn export(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ExportResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let now = state.clock.now();
    let wallet = state
        .database
        .wallet_for_session(&token, &now.to_rfc3339())
        .await?
        .ok_or_else(|| ApiError::unauthorized("session is invalid or expired"))?;
    let encrypted_answers = state.database.answers_for_wallet(&wallet).await?;
    let answers = encrypted_answers
        .into_iter()
        .map(|answer| {
            let plaintext = state.data_key.decrypt(&answer.nonce, &answer.ciphertext)?;
            serde_json::from_slice(&plaintext)
                .map_err(|_| ApiError::unavailable("stored answer cannot be decoded"))
        })
        .collect::<Result<Vec<StoredAnswer>, ApiError>>()?;

    Ok(Json(ExportResponse { answers }))
}

pub(crate) async fn erase(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let token = bearer_token(&headers)?;
    let now = state.clock.now();
    let wallet = state
        .database
        .wallet_for_session(&token, &now.to_rfc3339())
        .await?
        .ok_or_else(|| ApiError::unauthorized("session is invalid or expired"))?;
    state.database.delete_answers_for_wallet(&wallet).await?;

    Ok(StatusCode::NO_CONTENT)
}
