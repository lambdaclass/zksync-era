use validium_mode_example::{helpers::TxKind, scenario};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    scenario::run(20, 200, TxKind::Deploy).await;
    //scenario::basic().await;
}
