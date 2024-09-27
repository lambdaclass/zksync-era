use zksync_config::configs::da_client::eigen_da::EigenDAConfig;
use zksync_da_client::DataAvailabilityClient;
use zksync_da_clients::eigen_da::EigenDAClient;
use zksync_types::Address;

use crate::{
    implementations::resources::da_client::DAClientResource,
    wiring_layer::{WiringError, WiringLayer},
    IntoContext,
};

#[derive(Debug, Default)]
pub struct EigenDAWiringLayer {
    config: EigenDAConfig,
    verifier_address: Address,
}

impl EigenDAWiringLayer {
    pub fn new(config: EigenDAConfig, verifier_address: Address) -> Self {
        Self { config, verifier_address }
    }
}

#[derive(Debug, IntoContext)]
#[context(crate = crate)]
pub struct Output {
    pub client: DAClientResource,
}

#[async_trait::async_trait]
impl WiringLayer for EigenDAWiringLayer {
    type Input = ();
    type Output = Output;

    fn layer_name(&self) -> &'static str {
        "eigen_da_client_layer"
    }

    async fn wire(self, input: Self::Input) -> Result<Self::Output, WiringError> {
        let EthInterfaceResource(query_client) = input.eth_client;
        let client: Box<dyn DataAvailabilityClient> =
            Box::new(EigenDAClient::new(self.config,query_client, self.verifier_address).await?);

        Ok(Self::Output {
            client: DAClientResource(client),
        })
    }
}
