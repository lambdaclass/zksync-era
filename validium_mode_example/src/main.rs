use validium_mode_example::{helpers, scenario};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    scenario::run(5, 5, helpers::TxKind::Deploy).await;
}
