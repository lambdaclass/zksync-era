use zksync_config::{configs::da_client::eigen::EigenSecrets, EigenConfig};
use zksync_da_client::DataAvailabilityClient;
use zksync_da_clients::eigen::EigenDAClient;
use zksync_node_framework_derive::FromContext;

use crate::{
    implementations::resources::{
        da_client::DAClientResource,
        pools::{MasterPool, PoolResource},
    },
    wiring_layer::{WiringError, WiringLayer},
    IntoContext,
};

#[derive(Debug)]
pub struct EigenWiringLayer {
    config: EigenConfig,
    secrets: EigenSecrets,
}

impl EigenWiringLayer {
    pub fn new(config: EigenConfig, secrets: EigenSecrets) -> Self {
        Self { config, secrets }
    }
}

#[derive(Debug, FromContext)]
#[context(crate = crate)]
pub struct Input {
    pub master_pool: PoolResource<MasterPool>,
}

#[derive(Debug, IntoContext)]
#[context(crate = crate)]
pub struct Output {
    pub client: DAClientResource,
}

#[async_trait::async_trait]
impl WiringLayer for EigenWiringLayer {
    type Input = Input;
    type Output = Output;

    fn layer_name(&self) -> &'static str {
        "eigen_client_layer"
    }

    async fn wire(self, input: Self::Input) -> Result<Self::Output, WiringError> {
        let master_pool = input.master_pool.get().await?;
        let client: Box<dyn DataAvailabilityClient> =
            Box::new(EigenDAClient::new(self.config, self.secrets, master_pool).await?);

        Ok(Self::Output {
            client: DAClientResource(client),
        })
    }
}
