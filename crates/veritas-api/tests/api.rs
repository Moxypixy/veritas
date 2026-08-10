use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{TimeZone, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
use veritas::{VoteAnswer, commitment_hash};
use veritas_api::{
    ApiError, AppState, AuthVerifier, ChainVerifier, DataKey, Database, FixedClock, router,
};
use veritas_inflation::{OfficialPoint, OfficialSeries};

#[derive(Default)]
struct FakeChainVerifier {
    commitments: Mutex<HashMap<(String, String), [u8; 32]>>,
}

impl FakeChainVerifier {
    fn insert(&self, wallet: &str, month_key: &str, commitment: [u8; 32]) {
        self.commitments
            .lock()
            .unwrap()
            .insert((wallet.to_owned(), month_key.to_owned()), commitment);
    }
}

#[async_trait]
impl ChainVerifier for FakeChainVerifier {
    async fn commitment_for_vote(
        &self,
        wallet: &str,
        month_key: &str,
    ) -> Result<Option<[u8; 32]>, ApiError> {
        Ok(self
            .commitments
            .lock()
            .unwrap()
            .get(&(wallet.to_owned(), month_key.to_owned()))
            .copied())
    }
}

struct AcceptingAuthVerifier;

#[async_trait]
impl AuthVerifier for AcceptingAuthVerifier {
    async fn verify(
        &self,
        _wallet: &str,
        _challenge: &str,
        signature: &str,
    ) -> Result<bool, ApiError> {
        Ok(signature == "test-signature")
    }
}

async fn test_app(chain: Arc<FakeChainVerifier>) -> axum::Router {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    test_app_with_database(database, chain)
}

fn test_app_with_database(database: Database, chain: Arc<FakeChainVerifier>) -> axum::Router {
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 8, 10, 12, 0, 0).unwrap());
    router(AppState::new(
        database,
        DataKey::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap(),
        chain,
        Arc::new(AcceptingAuthVerifier),
        Arc::new(clock),
    ))
}

async fn json_response(response: axum::response::Response) -> Value {
    let body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

async fn post(app: axum::Router, path: &str, body: Value) -> axum::response::Response {
    app.oneshot(
        Request::post(path)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&body).unwrap()))
            .unwrap(),
    )
    .await
    .unwrap()
}

async fn authenticated_session(app: axum::Router, wallet: &str) -> String {
    let response = post(
        app.clone(),
        "/v1/auth/challenge",
        json!({ "wallet": wallet }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let challenge = json_response(response).await;

    let response = post(
        app,
        "/v1/auth/verify",
        json!({
            "wallet": wallet,
            "nonce": challenge["nonce"],
            "signature": "test-signature",
        }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    json_response(response).await["session_token"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn vote(wallet: &str, necessities_pct: u8, employed: bool) -> VoteAnswer {
    VoteAnswer {
        region_id: "euro-area".into(),
        necessities_pct,
        employed,
        duration_months: 12,
        month_key: "2026-08".into(),
        wallet: wallet.into(),
        salt: [9; 16],
    }
}

fn vote_request(answer: &VoteAnswer, commitment: [u8; 32]) -> Value {
    json!({
        "region_id": answer.region_id,
        "necessities_pct": answer.necessities_pct,
        "employed": answer.employed,
        "duration_months": answer.duration_months,
        "month_key": answer.month_key,
        "salt": hex::encode(answer.salt),
        "commitment": hex::encode(commitment),
        "tx_id": "local-runtime-vote"
    })
}

#[test]
fn data_key_round_trips_answer_bytes_with_a_fresh_nonce() {
    let key = DataKey::from_base64("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=").unwrap();
    let plaintext = br#"{"region_id":"euro-area"}"#;

    let (ciphertext, nonce) = key.encrypt(plaintext).unwrap();
    let decrypted = key.decrypt(&nonce, &ciphertext).unwrap();

    assert_ne!(ciphertext, plaintext);
    assert_eq!(decrypted, plaintext);
    assert_eq!(nonce.len(), 12);
}

#[tokio::test]
async fn challenge_verification_creates_a_wallet_bound_session() {
    let app = test_app(Arc::new(FakeChainVerifier::default())).await;
    let token = authenticated_session(app, "wallet-a").await;

    assert!(!token.is_empty());
}

#[tokio::test]
async fn vote_submission_rejects_a_declared_commitment_mismatch() {
    let chain = Arc::new(FakeChainVerifier::default());
    let app = test_app(chain.clone()).await;
    let wallet = "wallet-a";
    let answer = vote(wallet, 45, true);
    let chain_commitment = commitment_hash(&answer).unwrap();
    chain.insert(wallet, "2026-08", chain_commitment);
    let session = authenticated_session(app.clone(), wallet).await;

    let response = app
        .oneshot(
            Request::post("/v1/votes")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {session}"))
                .body(Body::from(
                    serde_json::to_vec(&vote_request(&answer, [0; 32])).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn aggregates_suppress_results_until_five_votes_exist() {
    let chain = Arc::new(FakeChainVerifier::default());
    let app = test_app(chain.clone()).await;

    for index in 0..4 {
        let wallet = format!("wallet-{index}");
        let answer = vote(&wallet, 40 + index, true);
        let commitment = commitment_hash(&answer).unwrap();
        chain.insert(&wallet, "2026-08", commitment);
        let session = authenticated_session(app.clone(), &wallet).await;

        let response = app
            .clone()
            .oneshot(
                Request::post("/v1/votes")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {session}"))
                    .body(Body::from(
                        serde_json::to_vec(&vote_request(&answer, commitment)).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let response = app
        .oneshot(
            Request::get("/v1/aggregates?region_id=euro-area&month_key=2026-08&employment=all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_response(response).await,
        json!({ "necessities_avg_pct": null, "n": 4, "suppressed": true })
    );
}

#[tokio::test]
async fn aggregates_suppress_all_related_bins_when_one_employment_group_is_sensitive() {
    let chain = Arc::new(FakeChainVerifier::default());
    let app = test_app(chain.clone()).await;

    for index in 0..6 {
        let wallet = format!("wallet-{index}");
        let answer = vote(&wallet, 40 + index, index < 5);
        let commitment = commitment_hash(&answer).unwrap();
        chain.insert(&wallet, "2026-08", commitment);
        let session = authenticated_session(app.clone(), &wallet).await;

        let response = app
            .clone()
            .oneshot(
                Request::post("/v1/votes")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {session}"))
                    .body(Body::from(
                        serde_json::to_vec(&vote_request(&answer, commitment)).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    for (employment, n) in [("all", 6), ("employed", 5), ("unemployed", 1)] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!(
                    "/v1/aggregates?region_id=euro-area&month_key=2026-08&employment={employment}"
                ))
                .body(Body::empty())
                .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            json_response(response).await,
            json!({ "necessities_avg_pct": null, "n": n, "suppressed": true })
        );
    }
}

#[tokio::test]
async fn chart_exposes_plain_language_official_price_series() {
    let database = Database::connect("sqlite::memory:").await.unwrap();
    database
        .replace_official_series(
            "euro-area",
            &[OfficialSeries {
                key: "prices_overall".into(),
                label: "How fast prices are rising overall".into(),
                points: vec![OfficialPoint {
                    month_key: "2026-08".into(),
                    value: 2.1,
                }],
            }],
        )
        .await
        .unwrap();
    let app = test_app_with_database(database, Arc::new(FakeChainVerifier::default()));

    let response = app
        .oneshot(
            Request::get("/v1/chart?region_id=euro-area")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_response(response).await,
        json!({
            "region_id": "euro-area",
            "official_unavailable": false,
            "series": [{
                "key": "prices_overall",
                "label": "How fast prices are rising overall",
                "points": [{ "month_key": "2026-08", "value": 2.1 }]
            }, {
                "key": "voters_necessities",
                "label": "What voters say they spend on necessities",
                "points": []
            }]
        })
    );
}

#[tokio::test]
async fn chart_marks_official_series_unavailable_without_a_successful_fetch() {
    let chain = Arc::new(FakeChainVerifier::default());
    let app = test_app(chain.clone()).await;

    for index in 0..5 {
        let wallet = format!("wallet-{index}");
        let answer = vote(&wallet, 40 + index, true);
        let commitment = commitment_hash(&answer).unwrap();
        chain.insert(&wallet, "2026-08", commitment);
        let session = authenticated_session(app.clone(), &wallet).await;
        let response = app
            .clone()
            .oneshot(
                Request::post("/v1/votes")
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::AUTHORIZATION, format!("Bearer {session}"))
                    .body(Body::from(
                        serde_json::to_vec(&vote_request(&answer, commitment)).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
    }

    let response = app
        .oneshot(
            Request::get("/v1/chart?region_id=euro-area")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_response(response).await,
        json!({
            "region_id": "euro-area",
            "official_unavailable": true,
            "series": [{
                "key": "voters_necessities",
                "label": "What voters say they spend on necessities",
                "points": [{ "month_key": "2026-08", "value": 42.0 }]
            }]
        })
    );
}

#[tokio::test]
async fn export_requires_a_wallet_bound_session() {
    let app = test_app(Arc::new(FakeChainVerifier::default())).await;

    let response = app
        .oneshot(Request::get("/v1/me/export").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn export_decrypts_only_the_session_wallet_answers() {
    let chain = Arc::new(FakeChainVerifier::default());
    let app = test_app(chain.clone()).await;
    let answer = vote("wallet-a", 45, true);
    let commitment = commitment_hash(&answer).unwrap();
    chain.insert("wallet-a", "2026-08", commitment);
    let session = authenticated_session(app.clone(), "wallet-a").await;

    let response = app
        .clone()
        .oneshot(
            Request::post("/v1/votes")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {session}"))
                .body(Body::from(
                    serde_json::to_vec(&vote_request(&answer, commitment)).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .oneshot(
            Request::get("/v1/me/export")
                .header(header::AUTHORIZATION, format!("Bearer {session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        json_response(response).await,
        json!({
            "answers": [{
                "region_id": "euro-area",
                "necessities_pct": 45,
                "employed": true,
                "duration_months": 12,
                "month_key": "2026-08",
                "salt": "09090909090909090909090909090909"
            }]
        })
    );
}

#[tokio::test]
async fn erase_removes_answers_without_recomputing_aggregate_bins() {
    let chain = Arc::new(FakeChainVerifier::default());
    let app = test_app(chain.clone()).await;
    let answer = vote("wallet-a", 45, true);
    let commitment = commitment_hash(&answer).unwrap();
    chain.insert("wallet-a", "2026-08", commitment);
    let session = authenticated_session(app.clone(), "wallet-a").await;

    let response = app
        .clone()
        .oneshot(
            Request::post("/v1/votes")
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::AUTHORIZATION, format!("Bearer {session}"))
                .body(Body::from(
                    serde_json::to_vec(&vote_request(&answer, commitment)).unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let response = app
        .clone()
        .oneshot(
            Request::delete("/v1/me/answers")
                .header(header::AUTHORIZATION, format!("Bearer {session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = app
        .clone()
        .oneshot(
            Request::get("/v1/me/export")
                .header(header::AUTHORIZATION, format!("Bearer {session}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(json_response(response).await, json!({ "answers": [] }));

    let response = app
        .oneshot(
            Request::get("/v1/aggregates?region_id=euro-area&month_key=2026-08&employment=all")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        json_response(response).await,
        json!({ "necessities_avg_pct": null, "n": 1, "suppressed": true })
    );
}
