use zksync_state::ReadStorage;

use super::traits::{Tracer, VmTracer};

pub struct RefundsTracer {}

impl RefundsTracer {
    pub fn new() -> Self {
        Self {}
    }
}

impl Tracer for RefundsTracer {}

impl<S: ReadStorage> VmTracer<S> for RefundsTracer {}
