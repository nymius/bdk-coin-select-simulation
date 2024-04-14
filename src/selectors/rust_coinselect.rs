use crate::selectors::TargetSelector;
use crate::{ SEGWIT_V1_TXIN_WEIGHT, SEGWIT_V1_TXOUT_WEIGHT };
use crate::models::{ PendingPayment, ScenarioEntry, SimulationEntry };

use std::{
    error::Error,
    collections::hash_set::HashSet,
};

use bitcoin::amount::Amount;

use rust_coinselect::{ OutputGroup, CoinSelectionOpt, ExcessStrategy, SelectionOutput, select_coin_fifo };

fn varint_size(v: usize) -> u32 {
    if v <= 0xfc {
        return 1;
    }
    if v <= 0xffff {
        return 3;
    }
    if v <= 0xffff_ffff {
        return 5;
    }
    9
}

fn effective_value(value: u64, weight: u32, sats_per_wu: f32) -> f32 {
    value as f32 - (weight as f32 * sats_per_wu)
}

#[derive(Default)]
pub struct RustCoinSelect {
    candidates: Vec<OutputGroup>,
    sequence_counter: u32,
    pub long_term_feerate: f32,
    pub dust_limit: u64,
    pub input_drain_weight: u32,
    pub output_drain_weight: u32,
}

impl RustCoinSelect {
    pub fn new(long_term_feerate: f32, dust_limit: u64, input_drain_weight: u32, output_drain_weight: u32) -> Self {
        RustCoinSelect {
            candidates: Vec::default(),
            sequence_counter: u32::default(),
            long_term_feerate,
            dust_limit,
            input_drain_weight,
            output_drain_weight,
        }
    }

    fn cost_to_empty_at_long_term_feerate(&self) -> f32 {
        self.candidates.len() as f32 * SEGWIT_V1_TXIN_WEIGHT as f32 * self.long_term_feerate
    }

    fn balance(&self) -> u64 {
        self.values().iter().sum::<u64>()
    }
}


impl TargetSelector for RustCoinSelect {
    fn values(&self) -> Vec<u64> {
        self.candidates.iter().map(|x| x.value).collect::<Vec<u64>>()
    }

    fn deposit(&mut self, deposit: ScenarioEntry) -> Result<(), Box<dyn Error>> {
        self.candidates.push(OutputGroup {
            creation_sequence: Some(self.sequence_counter),
            input_count: 1,
            weight: SEGWIT_V1_TXIN_WEIGHT,
            value: Amount::from_btc(deposit.amount)?.to_sat(),
            is_segwit: true
        });
        self.sequence_counter += 1;
        Ok(())
    }
    fn withdraw(&mut self, payments: &[PendingPayment], fee_rate_per_kvb: f32) -> SimulationEntry {
        let (output_count, output_weight_total) = payments
            .iter()
            .map(|x| x.weight)
            .fold((0_usize, 0_u32), |(n, w), a| (n + 1, w + a));

        let base_weight = (4 /* nVersion */
            + 4 /* nLockTime */
            + varint_size(0) /* inputs varint */
            + varint_size(output_count)/* outputs varint */)
            * 4
            + output_weight_total;

        let target_feerate = fee_rate_per_kvb * 1e5 / 4.0;

        let selection_options = CoinSelectionOpt {
            target_value: payments.iter().map(|x| x.amount).sum(),
            target_feerate,
            long_term_feerate: Some(self.long_term_feerate),
            min_absolute_fee: 0,
            base_weight,
            drain_weight: self.output_drain_weight,
            drain_cost: self.input_drain_weight as u64,
            cost_per_input: SEGWIT_V1_TXIN_WEIGHT as u64,
            cost_per_output: SEGWIT_V1_TXOUT_WEIGHT as u64,
            min_drain_value: 526,
            excess_strategy: ExcessStrategy::ToDrain
        };

        let mut withdraw: SimulationEntry = SimulationEntry {
            amount: selection_options.target_value,
            target_feerate,
            utxo_count_after_payment: self.candidates.len(),
            utxo_count_before_payment: self.candidates.len(),
            cost_to_empty_at_long_term_feerate: self.cost_to_empty_at_long_term_feerate(),
            balance: self.balance(),
            ..Default::default()
        };


        let selection = match select_coin_fifo(&self.candidates, selection_options) {
            Ok(SelectionOutput{ selected_inputs, .. }) => {
                withdraw.algorithm = String::from("fifo");
                selected_inputs
            },
            Err(_) => {
                withdraw.algorithm = String::from("failed");
                return withdraw
            },
        };

        let selection: HashSet<usize> = HashSet::from_iter(selection.iter().cloned());

        let mut selected_inputs = Vec::new();
        let mut not_selected_candidates = Vec::new();
        for (index, output_group) in self.candidates.iter().enumerate() {
            if selection.contains(&index) {
                selected_inputs.push(*output_group);
            } else {
                not_selected_candidates.push(*output_group);
            }
        }
        self.candidates = not_selected_candidates;

        let selected_value = selected_inputs.iter().map(|x| x.value).sum::<u64>();
        let input_weight = selected_inputs.iter().map(|x| x.weight).sum::<u32>();
        let total_weight = input_weight + base_weight + SEGWIT_V1_TXOUT_WEIGHT;
        let input_waste = input_weight as f32 * (selection_options.target_feerate - self.long_term_feerate);
        withdraw.negative_effective_valued_inputs = Some(selected_inputs.iter().filter(|x| effective_value(x.value, x.weight, selection_options.target_feerate) < 0.0).count());
        withdraw.inputs = selected_inputs.iter().map(|x| x.value).collect::<Vec<u64>>();
        let fee = (total_weight as f32 * selection_options.target_feerate) as i64;
        withdraw.fee = Some(fee);
        withdraw.change_amount = Some(selected_value - fee as u64);
        withdraw.waste_score = Some(input_waste + SEGWIT_V1_TXIN_WEIGHT as f32 * self.long_term_feerate + SEGWIT_V1_TXOUT_WEIGHT as f32 * selection_options.target_feerate);
        withdraw.real_feerate = if total_weight == 0 {
            None
        } else {
            Some(selected_value as f32 / total_weight as f32)
        };
        withdraw.output_count = Some(selected_inputs.len() + 1);
        withdraw.utxo_count_after_payment = self.candidates.len();
        withdraw
    }
}
