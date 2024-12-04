/// EigenDA Client tests are ignored by default, because they require a remote dependency,
/// which may not always be available, causing tests to be flaky.
/// To run these tests, use the following command:
/// `cargo test -p zksync_da_clients -- --ignored`
#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use serial_test::serial;
    use zksync_config::{
        configs::da_client::eigen::{EigenSecrets, PointsSource},
        EigenConfig,
    };
    use zksync_da_client::{types::DAError, DataAvailabilityClient};
    use zksync_types::secrets::PrivateKey;

    use crate::eigen::{blob_info::BlobInfo, EigenClient};

    impl EigenClient {
        pub async fn get_blob_data(
            &self,
            blob_id: BlobInfo,
        ) -> anyhow::Result<Option<Vec<u8>>, DAError> {
            self.client.get_blob_data(blob_id).await
        }

        pub async fn get_commitment(&self, blob_id: &str) -> anyhow::Result<BlobInfo> {
            self.client.get_commitment(blob_id).await
        }
    }

    #[ignore = "depends on external RPC"]
    #[tokio::test]
    #[serial]
    async fn test_non_auth_dispersal() {
        let config = EigenConfig {
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            settlement_layer_confirmation_depth: -1,
            eigenda_eth_rpc: "https://ethereum-holesky-rpc.publicnode.com".to_string(),
            eigenda_svc_manager_address: "0xD4A7E1Bd8015057293f0D0A557088c286942e84b".to_string(),
            status_query_timeout: 1800000, // 30 minutes
            status_query_interval: 5,      // 5 ms
            wait_for_finalization: false,
            authenticated: false,
            points_source: PointsSource::Path("../../../resources".to_string()),
            chain_id: 17000,
        };
        let secrets = EigenSecrets {
            private_key: PrivateKey::from_str(
                "d08aa7ae1bb5ddd46c3c2d8cdb5894ab9f54dec467233686ca42629e826ac4c6",
            )
            .unwrap(),
        };
        let client = EigenClient::new(config, secrets).await.unwrap();
        let data = vec![1; 20];
        let result = client.dispatch_blob(0, data.clone()).await.unwrap();

        let blob_info = client.get_commitment(&result.blob_id).await.unwrap();
        let expected_inclusion_data = blob_info.clone().blob_verification_proof.inclusion_proof;
        let actual_inclusion_data = client
            .get_inclusion_data(&result.blob_id)
            .await
            .unwrap()
            .unwrap()
            .data;
        assert_eq!(expected_inclusion_data, actual_inclusion_data);
        let retrieved_data = client.get_blob_data(blob_info).await.unwrap();
        assert_eq!(retrieved_data.unwrap(), data);
    }

    #[ignore = "depends on external RPC"]
    #[tokio::test]
    #[serial]
    async fn test_auth_dispersal() {
        let config = EigenConfig {
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            settlement_layer_confirmation_depth: -1,
            eigenda_eth_rpc: "https://ethereum-holesky-rpc.publicnode.com".to_string(),
            eigenda_svc_manager_address: "0xD4A7E1Bd8015057293f0D0A557088c286942e84b".to_string(),
            status_query_timeout: 1800000, // 30 minutes
            status_query_interval: 5,      // 5 ms
            wait_for_finalization: false,
            authenticated: true,
            points_source: PointsSource::Path("../../../resources".to_string()),
            chain_id: 17000,
        };
        let secrets = EigenSecrets {
            private_key: PrivateKey::from_str(
                "d08aa7ae1bb5ddd46c3c2d8cdb5894ab9f54dec467233686ca42629e826ac4c6",
            )
            .unwrap(),
        };
        let client = EigenClient::new(config, secrets).await.unwrap();
        let data = vec![1; 20];
        let result = client.dispatch_blob(0, data.clone()).await.unwrap();
        let blob_info = client.get_commitment(&result.blob_id).await.unwrap();

        let expected_inclusion_data = blob_info.clone().blob_verification_proof.inclusion_proof;
        let actual_inclusion_data = client
            .get_inclusion_data(&result.blob_id)
            .await
            .unwrap()
            .unwrap()
            .data;
        assert_eq!(expected_inclusion_data, actual_inclusion_data);
        let retrieved_data = client.get_blob_data(blob_info).await.unwrap();
        assert_eq!(retrieved_data.unwrap(), data);
    }

    #[ignore = "depends on external RPC"]
    #[tokio::test]
    #[serial]
    async fn test_wait_for_finalization() {
        let config = EigenConfig {
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            status_query_timeout: 1800000, // 30 minutes
            status_query_interval: 5000,   // 5000 ms
            wait_for_finalization: true,
            authenticated: true,
            points_source: PointsSource::Path("../../../resources".to_string()),
            settlement_layer_confirmation_depth: 0,
            eigenda_eth_rpc: "https://ethereum-holesky-rpc.publicnode.com".to_string(),
            eigenda_svc_manager_address: "0xD4A7E1Bd8015057293f0D0A557088c286942e84b".to_string(),
            chain_id: 17000,
        };
        let secrets = EigenSecrets {
            private_key: PrivateKey::from_str(
                "d08aa7ae1bb5ddd46c3c2d8cdb5894ab9f54dec467233686ca42629e826ac4c6",
            )
            .unwrap(),
        };
        let client = EigenClient::new(config, secrets).await.unwrap();
        let data = vec![1; 20];
        let result = client.dispatch_blob(0, data.clone()).await.unwrap();
        let blob_info = client.get_commitment(&result.blob_id).await.unwrap();

        let expected_inclusion_data = blob_info.clone().blob_verification_proof.inclusion_proof;
        let actual_inclusion_data = client
            .get_inclusion_data(&result.blob_id)
            .await
            .unwrap()
            .unwrap()
            .data;
        assert_eq!(expected_inclusion_data, actual_inclusion_data);
        let retrieved_data = client.get_blob_data(blob_info).await.unwrap();
        assert_eq!(retrieved_data.unwrap(), data);
    }

    #[ignore = "depends on external RPC"]
    #[tokio::test]
    #[serial]
    async fn test_settlement_layer_confirmation_depth() {
        let config = EigenConfig {
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            settlement_layer_confirmation_depth: 5,
            eigenda_eth_rpc: "https://ethereum-holesky-rpc.publicnode.com".to_string(),
            eigenda_svc_manager_address: "0xD4A7E1Bd8015057293f0D0A557088c286942e84b".to_string(),
            status_query_timeout: 1800000, // 30 minutes
            status_query_interval: 5,      // 5 ms
            wait_for_finalization: false,
            authenticated: false,
            points_source: PointsSource::Path("../../../resources".to_string()),
            chain_id: 17000,
        };
        let secrets = EigenSecrets {
            private_key: PrivateKey::from_str(
                "d08aa7ae1bb5ddd46c3c2d8cdb5894ab9f54dec467233686ca42629e826ac4c6",
            )
            .unwrap(),
        };
        let client = EigenClient::new(config, secrets).await.unwrap();
        let data = vec![1; 20];
        let result = client.dispatch_blob(0, data.clone()).await.unwrap();
        let blob_info = client.get_commitment(&result.blob_id).await.unwrap();

        let expected_inclusion_data = blob_info.clone().blob_verification_proof.inclusion_proof;
        let actual_inclusion_data = client
            .get_inclusion_data(&result.blob_id)
            .await
            .unwrap()
            .unwrap()
            .data;
        assert_eq!(expected_inclusion_data, actual_inclusion_data);
        let retrieved_data = client.get_blob_data(blob_info).await.unwrap();
        assert_eq!(retrieved_data.unwrap(), data);
    }

    #[ignore = "depends on external RPC"]
    #[tokio::test]
    #[serial]
    async fn test_auth_dispersal_settlement_layer_confirmation_depth() {
        let config = EigenConfig {
            disperser_rpc: "https://disperser-holesky.eigenda.xyz:443".to_string(),
            settlement_layer_confirmation_depth: 5,
            eigenda_eth_rpc: "https://ethereum-holesky-rpc.publicnode.com".to_string(),
            eigenda_svc_manager_address: "0xD4A7E1Bd8015057293f0D0A557088c286942e84b".to_string(),
            status_query_timeout: 1800000, // 30 minutes
            status_query_interval: 5,      // 5 ms
            wait_for_finalization: false,
            authenticated: true,
            points_source: PointsSource::Path("../../../resources".to_string()),
            chain_id: 17000,
        };
        let secrets = EigenSecrets {
            private_key: PrivateKey::from_str(
                "d08aa7ae1bb5ddd46c3c2d8cdb5894ab9f54dec467233686ca42629e826ac4c6",
            )
            .unwrap(),
        };
        let client = EigenClient::new(config, secrets).await.unwrap();
        let data = vec![1; 20];
        let result = client.dispatch_blob(0, data.clone()).await.unwrap();
        let blob_info = client.get_commitment(&result.blob_id).await.unwrap();

        let expected_inclusion_data = blob_info.clone().blob_verification_proof.inclusion_proof;
        let actual_inclusion_data = client
            .get_inclusion_data(&result.blob_id)
            .await
            .unwrap()
            .unwrap()
            .data;
        assert_eq!(expected_inclusion_data, actual_inclusion_data);
        let retrieved_data = client.get_blob_data(blob_info).await.unwrap();
        assert_eq!(retrieved_data.unwrap(), data);
    }
}
