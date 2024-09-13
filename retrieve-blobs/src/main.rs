use reqwest::header::CONTENT_TYPE;
use tokio_postgres::{NoTls, Error};
use prometheus::{Counter, Histogram, Encoder, TextEncoder, register_counter, register_histogram};
use hyper::{Body, Response, Server, Request};
use hyper::service::{make_service_fn, service_fn};
use std::collections::HashSet;
use std::convert::Infallible;
use std::time::Instant;
use tokio::time::sleep;
use std::time::Duration;

// Define Prometheus metrics
lazy_static::lazy_static! {
    static ref BLOB_RETRIEVALS: Counter = register_counter!(
        "blob_retrievals_total", 
        "Total number of blobs successfully retrieved"
    ).unwrap();

    static ref BLOB_RETRIEVAL_ERRORS: Counter = register_counter!(
        "blob_retrieval_errors_total", 
        "Total number of failed blob retrievals"
    ).unwrap();
}

#[tokio::main]
async fn main() {
    // Start the metrics HTTP server
    tokio::spawn(start_metrics_server());

    let mut blobs = HashSet::new();

    loop {
        // Perform blob retrievals
        get_blobs(&mut blobs).await.unwrap();
    }
}

async fn get_blobs(blobs: &mut HashSet<String>) -> Result<(), Error> {
    // Connect to the PostgreSQL server
    let (client, connection) =
        tokio_postgres::connect("host=localhost user=postgres password=notsecurepassword dbname=zksync_local", NoTls).await?;

    // Spawn a background task to handle the connection
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    // Run the SELECT query
    let rows = client
        .query("SELECT blob_id FROM data_availability", &[])
        .await?;

    for row in rows {
        let blob_id: &str = row.get(0);
        let blob_id = blob_id.to_string();
        
        if !blobs.contains(&blob_id) {
            blobs.insert(blob_id.clone());
            let blob = get(blob_id).await;

            if blob.is_empty(){
                BLOB_RETRIEVAL_ERRORS.inc();  // Increment error counter if blob retrieval fails
            } else {
                BLOB_RETRIEVALS.inc();  // Increment success counter if blob retrieval succeeds
            }
        }

    }

    Ok(())
}

async fn get(commitment: String) -> Vec<u8> {
    let url = format!("http://127.0.0.1:4242/get/0x{commitment}");

    let client = reqwest::Client::new();
    let response = client.get(url).send().await.unwrap();

    if response.status().is_success() {
        // Expecting the response body to be binary data
        let body = response.bytes().await.unwrap();
        body.to_vec()
    } else {
        vec![]
    }
}

// Start the Prometheus metrics server
async fn start_metrics_server() {
    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(metrics_handler))
    });

    let addr = ([127, 0, 0, 1], 7070).into();
    let server = Server::bind(&addr).serve(make_svc);

    println!("Serving metrics on http://{}", addr);
    server.await.unwrap();
}

// Handle the /metrics endpoint
async fn metrics_handler(_: Request<Body>) -> Result<Response<Body>, Infallible> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Ok(Response::new(Body::from(buffer)))
}
