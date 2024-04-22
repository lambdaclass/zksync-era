use validium_mode_example::{helpers::TxKind, scenario};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    scenario::run(1, 1, TxKind::Deploy).await;
    // scenario::basic().await;
    // scenario::deploy_erc20(20, 200).await;
    // scenario::mint_erc20(20, 200).await;
    // scenario::transfer_erc20(20, 200).await;
}