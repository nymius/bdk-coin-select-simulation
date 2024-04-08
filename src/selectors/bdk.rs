use crate::SEGWIT_V1_TXIN_WEIGHT;
use crate::selectors::TargetSelector;
use crate::models::{ PendingPayment, ScenarioEntry, SimulationEntry };

use std::error::Error;

use bitcoin::amount::Amount;

use bdk_coin_select::{ Candidate, CoinSelector, FeeRate, Target, TargetFee, ChangePolicy, DrainWeights };
use bdk_coin_select::metrics::LowestFee;

pub struct BdkCoinSelect {
    candidates: Vec<Candidate>,
    pub long_term_feerate: f32,
    pub dust_limit: u64,
    pub input_drain_weight: u32,
    pub output_drain_weight: u32,
}

impl BdkCoinSelect {
    pub fn new(long_term_feerate: f32, dust_limit: u64, input_drain_weight: u32, output_drain_weight: u32) -> Self {
        BdkCoinSelect {
            candidates: Vec::default(),
            long_term_feerate,
            dust_limit,
            input_drain_weight,
            output_drain_weight,
        }
    }
}

impl BdkCoinSelect {
    fn cost_to_empty_at_long_term_feerate(&self) -> f32 {
        self.candidates.len() as f32 * SEGWIT_V1_TXIN_WEIGHT as f32 * self.long_term_feerate
    }

    fn balance(&self) -> u64 {
        self.values().iter().sum::<u64>()
    }
}

impl TargetSelector for BdkCoinSelect {
    fn values(&self) -> Vec<u64> {
        self.candidates.iter().map(|x| x.value).collect::<Vec<u64>>()
    }

    fn deposit(&mut self, record: ScenarioEntry) -> Result<(), Box<dyn Error>> {
        self.candidates.push(Candidate {
            input_count: 1,
            weight: SEGWIT_V1_TXIN_WEIGHT,
            value: Amount::from_btc(record.amount)?.to_sat(),
            is_segwit: true
        });
        Ok(())
    }

    fn withdraw(&mut self, payments: &[PendingPayment], fee_rate_per_kvb: f32) -> SimulationEntry {
        let selection_inputs = self.candidates.clone();

        let mut coin_selector = CoinSelector::fund_outputs(&selection_inputs, payments.iter().map(|x| x.weight));

        let drain_weights = DrainWeights { output_weight: self.input_drain_weight, spend_weight: self.output_drain_weight };
        let target = Target {
            fee: TargetFee::from_feerate(FeeRate::from_btc_per_kvb(fee_rate_per_kvb)),
            value: payments.iter().map(|x| x.amount).sum::<u64>(),
        };

        let mut withdraw: SimulationEntry = SimulationEntry {
            amount: target.value,
            target_feerate: target.fee.rate.as_sat_vb(),
            utxo_count_after_payment: self.candidates.len(),
            utxo_count_before_payment: self.candidates.len(),
            cost_to_empty_at_long_term_feerate: self.cost_to_empty_at_long_term_feerate(),
            balance: self.balance(),
            ..Default::default()
        };

        if !coin_selector.is_selection_possible(target) {
            withdraw.algorithm = String::from("failed");
            return withdraw
        }

        let long_term_feerate = FeeRate::from_sat_per_vb(self.long_term_feerate);
        // We use a change policy that introduces a change output if doing so reduces
        // the "waste" and that the change output's value is at least that of the
        // `dust_limit`.
        let change_policy = ChangePolicy::min_value_and_waste(
            drain_weights,
            self.dust_limit,
            target.fee.rate,
            long_term_feerate,
        );

        // This metric minimizes transaction fees paid over time. The
        // `long_term_feerate` is used to calculate the additional fee from spending
        // the change output in the future.
        let metric = LowestFee {
            target,
            long_term_feerate,
            change_policy
        };

        

        // We run the branch and bound algorithm with a max round limit of 100,000.
        match coin_selector.run_bnb(metric, 100_000) {
            Err(err) => {
                println!("failed to find a solution: {}", err);
                // fall back to naive selection
                coin_selector.select_until_target_met(target).expect("a selection was impossible!");
                withdraw.algorithm = String::from("select_sorted");
            }
            Ok(score) => {
                withdraw.algorithm = String::from("bnb");
                println!("we found a solution with score {}", score);
            }
        };

        let change = coin_selector.drain(target, change_policy);

        let selection = coin_selector
            .apply_selection(&selection_inputs)
            .collect::<Vec<_>>();


        self.candidates = coin_selector.unselected().map(|x| x.1).collect::<Vec<_>>();

        println!("we selected {} inputs", selection.len());
        println!("We are including a change output of {} value (0 means not change)", change.value);

        self.candidates.push(Candidate {
            input_count: 1,
            weight: SEGWIT_V1_TXIN_WEIGHT,
            value: change.value,
            is_segwit: true
        });

        withdraw.negative_effective_valued_inputs = Some(coin_selector.selected().filter(|x| x.1.effective_value(target.fee.rate) < 0.0).count());
        withdraw.inputs = coin_selector.selected().map(|x| x.1.value).collect::<Vec<u64>>();
        withdraw.fee = Some(coin_selector.fee(target.value, change.value));
        withdraw.real_feerate = Some(coin_selector.implied_feerate(target.value, change).expect("selection is finished").as_sat_vb());
        withdraw.output_count = Some(if change.value != 0 {
            payments.len() + 1_usize
        } else {
            payments.len()
        });
        withdraw.change_amount = if change.value > 0 {
            Some(change.value)
        } else { None };
        withdraw.utxo_count_after_payment = self.candidates.len();
        withdraw.waste_score = Some(coin_selector.waste(target, long_term_feerate, change, 1.0));
        withdraw
    }
}
