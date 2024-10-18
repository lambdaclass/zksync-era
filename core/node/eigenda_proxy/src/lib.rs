use std::net::SocketAddr;

use anyhow::Context as _;
use axum::{
    extract::Path,
    routing::{get, post},
    Router,
};
use disperser::disperser_client::DisperserClient;
use request_processor::RequestProcessor;
use tokio::sync::watch;
use tonic::transport::{Channel, ClientTlsConfig};

mod blob_info;
mod common;
mod disperser;
mod errors;
mod memstore;
mod request_processor;

pub async fn run_server(mut stop_receiver: watch::Receiver<bool>) -> anyhow::Result<()> {
    // TODO: Replace port for config
    let bind_address = SocketAddr::from(([0, 0, 0, 0], 4242));
    tracing::info!("Starting eigenda proxy on {bind_address}");

    let disperser_endpoint = Channel::builder(
        "https://disperser-holesky.eigenda.xyz"
            .to_string()
            .parse()
            .unwrap(),
    )
    .tls_config(ClientTlsConfig::new().with_native_roots())
    .unwrap();
    let disperser = DisperserClient::connect(disperser_endpoint).await.unwrap();
    let app = create_eigenda_proxy_router(disperser);

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

fn create_eigenda_proxy_router(disperser: DisperserClient<Channel>) -> Router {
    let get_blob_id_processor = RequestProcessor::new(disperser);
    let _put_blob_id_processor = get_blob_id_processor.clone();
    let router = Router::new()
        .route(
            "/get/:l1_batch_number",
            get(move |blob_id: Path<String>| async move {
                get_blob_id_processor.get_blob_id(blob_id).await
            }),
        )
        .route(
            "/put/",
            post(move |_blob_id: Path<u32>| async move {
                // put_blob_id_processor
                //     .put_blob_id(blob_id)
                //     .await
            }),
        );
    router
}
