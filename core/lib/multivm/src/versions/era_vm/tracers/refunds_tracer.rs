use zksync_state::ReadStorage;
use zksync_types::{H256, U256};
use zksync_utils::ceil_div_u256;

use super::traits::{Tracer, VmTracer};
use crate::{
    era_vm::hook::Hook,
    vm_latest::{
        constants::{OPERATOR_REFUNDS_OFFSET, TX_GAS_LIMIT_OFFSET},
        utils::fee::get_batch_base_fee,
        L1BatchEnv, Refunds,
    },
};

#[derive(Default)]
pub struct RefundsTracer {
    pub gas_refunded: u64,
    pub operator_suggested_refund: u64,
    pubdata_before: u32,
}

impl RefundsTracer {
    pub fn new() -> Self {
        Self {
            gas_refunded: 0,
            operator_suggested_refund: 0,
            pubdata_before: 0,
        }
    }
}

impl Into<Refunds> for RefundsTracer {
    fn into(self) -> Refunds {
        Refunds {
            gas_refunded: self.gas_refunded,
            operator_suggested_refund: self.operator_suggested_refund,
        }
    }
}

impl Tracer for RefundsTracer {}

impl<S: ReadStorage> VmTracer<S> for RefundsTracer {
    fn before_bootloader_execution(&mut self, vm: &mut super::traits::Vm<S>) {
        self.pubdata_before = vm.inner.state.pubdata() as u32;
    }

    fn bootloader_hook_call(
        &mut self,
        vm: &mut super::traits::Vm<S>,
        hook: crate::era_vm::hook::Hook,
        hook_params: &[zksync_types::U256; 3],
    ) {
        match hook {
            Hook::NotifyAboutRefund => self.gas_refunded = hook_params[0].low_u64(),
            Hook::AskOperatorForRefund => {
                let [bootloader_refund, gas_spent_on_pubdata, gas_per_pubdata_byte] = hook_params;
                let current_tx_index = vm.bootloader_state.current_tx();
                let tx_description_offset = vm
                    .bootloader_state
                    .get_tx_description_offset(current_tx_index);
                let tx_gas_limit = vm
                    .read_heap_word(tx_description_offset + TX_GAS_LIMIT_OFFSET)
                    .as_u64();

                let pubdata_published = vm.inner.state.pubdata() as u32;

                self.operator_suggested_refund = compute_refund(
                    &vm.batch_env,
                    bootloader_refund.as_u64(),
                    gas_spent_on_pubdata.as_u64(),
                    tx_gas_limit,
                    gas_per_pubdata_byte.low_u32(),
                    pubdata_published.saturating_sub(self.pubdata_before),
                    vm.bootloader_state.last_l2_block().txs.last().unwrap().hash,
                );

                self.pubdata_before = pubdata_published;
                let refund_value = self.operator_suggested_refund;
                vm.write_to_bootloader_heap([(
                    OPERATOR_REFUNDS_OFFSET + current_tx_index,
                    refund_value.into(),
                )]);
                vm.bootloader_state.set_refund_for_current_tx(refund_value);
            }
            _ => {}
        };
    }
}

pub(crate) fn compute_refund(
    l1_batch: &L1BatchEnv,
    bootloader_refund: u64,
    gas_spent_on_pubdata: u64,
    tx_gas_limit: u64,
    current_ergs_per_pubdata_byte: u32,
    pubdata_published: u32,
    tx_hash: H256,
) -> u64 {
    let total_gas_spent = tx_gas_limit - bootloader_refund;

    let gas_spent_on_computation = total_gas_spent
        .checked_sub(gas_spent_on_pubdata)
        .unwrap_or_else(|| {
            tracing::error!(
                "Gas spent on pubdata is greater than total gas spent. On pubdata: {}, total: {}",
                gas_spent_on_pubdata,
                total_gas_spent
            );
            0
        });

    // For now, bootloader charges only for base fee.
    let effective_gas_price = get_batch_base_fee(l1_batch);

    let bootloader_eth_price_per_pubdata_byte =
        U256::from(effective_gas_price) * U256::from(current_ergs_per_pubdata_byte);

    let fair_eth_price_per_pubdata_byte = U256::from(l1_batch.fee_input.fair_pubdata_price());

    // For now, L1 originated transactions are allowed to pay less than fair fee per pubdata,
    // so we should take it into account.
    let eth_price_per_pubdata_byte_for_calculation = std::cmp::min(
        bootloader_eth_price_per_pubdata_byte,
        fair_eth_price_per_pubdata_byte,
    );

    let fair_fee_eth = U256::from(gas_spent_on_computation)
        * U256::from(l1_batch.fee_input.fair_l2_gas_price())
        + U256::from(pubdata_published) * eth_price_per_pubdata_byte_for_calculation;
    let pre_paid_eth = U256::from(tx_gas_limit) * U256::from(effective_gas_price);
    let refund_eth = pre_paid_eth.checked_sub(fair_fee_eth).unwrap_or_else(|| {
        tracing::error!(
            "Fair fee is greater than pre paid. Fair fee: {} wei, pre paid: {} wei",
            fair_fee_eth,
            pre_paid_eth
        );
        U256::zero()
    });

    tracing::trace!(
        "Fee benchmark for transaction with hash {}",
        hex::encode(tx_hash.as_bytes())
    );
    tracing::trace!("Gas Limit: {}", tx_gas_limit);
    tracing::trace!("Gas spent on computation: {}", gas_spent_on_computation);
    tracing::trace!("Gas spent on pubdata: {}", gas_spent_on_pubdata);
    tracing::trace!("Pubdata published: {}", pubdata_published);

    ceil_div_u256(refund_eth, effective_gas_price.into()).as_u64()
}
