use zksync_state::ReadStorage;

use super::traits::{Tracer, VmTracer};

pub struct ResultTracer {}

impl ResultTracer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tracer for ResultTracer {}

impl<S: ReadStorage> VmTracer<S> for ResultTracer {}
