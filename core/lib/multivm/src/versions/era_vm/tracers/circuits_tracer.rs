use zksync_state::ReadStorage;

use super::traits::{Tracer, VmTracer};

pub struct CircuitsTracer {}

impl CircuitsTracer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tracer for CircuitsTracer {}

impl<S: ReadStorage> VmTracer<S> for CircuitsTracer {}
