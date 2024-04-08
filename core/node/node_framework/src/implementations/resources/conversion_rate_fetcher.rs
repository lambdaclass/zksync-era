use std::sync::Arc;

use zksync_core::base_token_fetcher::ConversionRateFetcher;

use crate::resource::{Resource, ResourceId};

#[derive(Debug, Clone)]
pub struct ConversionRateFetcherResource(pub Arc<dyn ConversionRateFetcher>);

impl Resource for ConversionRateFetcherResource {
    fn resource_id() -> ResourceId {
        "common/conversion_rate_fetcher".into()
    }
}
