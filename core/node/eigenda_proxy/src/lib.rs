use std::net::SocketAddr;
use anyhow::Context as _;
use axum::{
    extract::Path,
    routing::{get, post, put},
    Json, Router,
};
use request_processor::RequestProcessor;
use tokio::sync::watch;

mod errors;
mod request_processor;
mod common;
mod disperser;

pub async fn run_server(mut stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
    // TODO: Replace port for config
    let bind_address = SocketAddr::from(([0, 0, 0, 0], 4242));
    tracing::info!("Starting eigenda proxy on {bind_address}");
    let app = create_eigenda_proxy_router();

    let listener = tokio::net::TcpListener::bind(bind_address)
        .await
        .with_context(|| format!("Failed binding eigenda proxy to {bind_address}"))?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            if stop_receiver.changed().await.is_err() {
                tracing::warn!(
                    "Stop signal sender for eigenda proxy was dropped without sending a signal"
                );
            }
            tracing::info!("Stop signal received, eigenda proxy is shutting down");
        })
        .await
        .context("EigenDA proxy failed")?;
    tracing::info!("EigenDA proxy shut down");
    Ok(())
}

fn create_eigenda_proxy_router() -> Router {
    let get_blob_id_processor = RequestProcessor::new();
    let put_blob_id_processor = get_blob_id_processor.clone();
    let mut router = Router::new()
        .route(
            "/get/:l1_batch_number",
            get(move |blob_id: Path<String>| async move {
                get_blob_id_processor.get_blob_id(blob_id).await
            }),
        )
        .route(
            "/put/",
            put(move |blob_id: Path<u32>| async move {
                // put_blob_id_processor
                //     .put_blob_id(blob_id)
                //     .await
            }),
        );
    router
}
