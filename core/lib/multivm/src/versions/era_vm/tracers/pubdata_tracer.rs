use zksync_state::ReadStorage;

use super::traits::{Tracer, VmTracer};

pub struct PubdataTracer {}

impl PubdataTracer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tracer for PubdataTracer {}

impl<S: ReadStorage> VmTracer<S> for PubdataTracer {}
