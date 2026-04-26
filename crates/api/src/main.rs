use anyhow::Context;
use novelgraph_ai::{LlamaCppClient, LlamaCppConfig};
use novelgraph_api::build_router;
use novelgraph_core::AppConfig;
use novelgraph_storage::SqliteStore;
use tokio::net::TcpListener;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "novelgraph_api=info,tower_http=info".into()),
        )
        .init();

    let config = AppConfig::from_env()?;
    let store = if let Some(database_url) = &config.database_url {
        if !database_url.starts_with("sqlite:") {
            anyhow::bail!(
                "only SQLite storage is implemented in this foundation slice; set SQLITE_DATABASE_PATH or use a sqlite DATABASE_URL"
            );
        }
        SqliteStore::connect_url(database_url).await?
    } else {
        SqliteStore::connect(config.sqlite_database_path.as_deref()).await?
    };
    let local_llm = LlamaCppClient::new(LlamaCppConfig {
        base_url: config.llama_cpp_base_url.clone(),
        default_model: config.llama_cpp_default_model.clone(),
        timeout_secs: config.llama_cpp_timeout_secs,
    })?;
    let bind_addr = config.bind_addr();
    let router = build_router(config, store, local_llm);
    let listener = TcpListener::bind(&bind_addr)
        .await
        .with_context(|| format!("failed to bind {bind_addr}"))?;

    info!(%bind_addr, "novelgraph api listening");
    axum::serve(listener, router).await?;
    Ok(())
}
