use std::env;

use sqlx::sqlite::SqlitePoolOptions;
use veritas_inflation::{
    RefreshResult, ReqwestHttpClient, fetch_region, migrate_store, parse_config,
};

const REGIONS_JSON: &str = include_str!("../regions.json");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL must name the SQLite database used by the API")?;
    let config = parse_config(REGIONS_JSON)?;
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&database_url)
        .await?;
    migrate_store(&pool).await?;

    let client = ReqwestHttpClient::default();
    let mut failed_regions = 0;
    for region in &config.regions {
        match fetch_region(&client, &pool, &config.source, region).await {
            Ok(RefreshResult::Updated) => {
                println!("updated {}", region.id);
            }
            Ok(RefreshResult::Skipped) => {
                println!("skipped {}: required series are not configured", region.id);
            }
            Err(error) => {
                failed_regions += 1;
                eprintln!("failed {}: {error}", region.id);
            }
        }
    }

    if failed_regions == 0 {
        Ok(())
    } else {
        Err(format!("{failed_regions} official-statistics refreshes failed").into())
    }
}
