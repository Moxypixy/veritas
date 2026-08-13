use sqlx::sqlite::SqlitePoolOptions;
use veritas_inflation::{
    HttpClient, RefreshResult, fetch_region, migrate_store, parse_config, region_data,
};

const RECORDED_OECD_RESPONSE: &str = include_str!("fixtures/oecd-prices.csv");

struct RecordedHttpClient;

impl HttpClient for RecordedHttpClient {
    async fn get(&self, _url: &str) -> Result<String, String> {
        Ok(RECORDED_OECD_RESPONSE.to_owned())
    }
}

struct FailingHttpClient;

impl HttpClient for FailingHttpClient {
    async fn get(&self, _url: &str) -> Result<String, String> {
        Err("OECD is unavailable".into())
    }
}

async fn store() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    migrate_store(&pool).await.unwrap();
    pool
}

#[tokio::test]
async fn worker_stores_all_required_series_from_recorded_http_responses() {
    let config = parse_config(include_str!("../regions.json")).unwrap();
    let region = config
        .regions
        .iter()
        .find(|region| region.id == "euro-area")
        .unwrap();
    let pool = store().await;

    let result = fetch_region(&RecordedHttpClient, &pool, &config.source, region)
        .await
        .unwrap();

    assert_eq!(result, RefreshResult::Updated);
    let stored = region_data(&pool, "euro-area").await.unwrap();
    assert!(!stored.unavailable);
    assert_eq!(stored.series.len(), 4);
    assert!(stored.series.iter().all(|series| series.points.len() == 2));
}

#[tokio::test]
async fn failed_fetch_marks_official_series_unavailable() {
    let config = parse_config(include_str!("../regions.json")).unwrap();
    let region = config
        .regions
        .iter()
        .find(|region| region.id == "euro-area")
        .unwrap();
    let pool = store().await;

    let result = fetch_region(&FailingHttpClient, &pool, &config.source, region).await;

    assert!(result.is_err());
    assert!(region_data(&pool, "euro-area").await.unwrap().unavailable);
}
