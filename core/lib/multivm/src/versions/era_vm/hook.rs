#[derive(Debug, Clone)]

pub enum Hook {
    AccountValidationEntered,
    PaymasterValidationEntered,
    AccountValidationExited,
    ValidationStepEnded,
    TxHasEnded,
    DebugLog,
    DebugReturnData,
    NearCallCatch,
    AskOperatorForRefund,
    NotifyAboutRefund,
    PostResult,
    FinalBatchInfo,
    PubdataRequested,
    LoadParallel,
    TxIndex,
}

impl Hook {
    /// # Panics
    /// Panics if the number does not correspond to any hook.
    pub fn from_u32(hook: u32) -> Self {
        match hook {
            0 => Hook::AccountValidationEntered,
            1 => Hook::PaymasterValidationEntered,
            2 => Hook::AccountValidationExited,
            3 => Hook::ValidationStepEnded,
            4 => Hook::TxHasEnded,
            5 => Hook::DebugLog,
            6 => Hook::DebugReturnData,
            7 => Hook::NearCallCatch,
            8 => Hook::AskOperatorForRefund,
            9 => Hook::NotifyAboutRefund,
            10 => Hook::PostResult,
            11 => Hook::FinalBatchInfo,
            12 => Hook::PubdataRequested,
            13 => Hook::LoadParallel,
            14 => Hook::TxIndex,
            _ => panic!("Unknown hook {}", hook),
        }
    }
}
