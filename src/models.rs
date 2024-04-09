use std::{
    cmp,
    error::Error,
    collections::HashMap,
};
use serde::ser::{ Serialize, Serializer, SerializeStruct };
use statistical::{ mean, standard_deviation };

#[derive(Debug, serde::Deserialize, Clone)]
pub struct ScenarioEntry {
    pub amount: f64,
    pub fee_rate_per_kvb: f32,
}

pub struct PendingPayment {
    pub amount: u64,
    pub weight: u32,
}

#[derive(Debug, Default)]
pub struct SimulationEntry {
    pub id: usize,
    pub inputs: Vec<u64>,
    pub amount: u64,
    pub fee: Option<i64>,
    pub target_feerate: f32,
    pub real_feerate: Option<f32>,
    pub algorithm: String,
    pub negative_effective_valued_inputs: Option<usize>,
    pub output_count: Option<usize>,
    pub change_amount: Option<u64>,
    pub utxo_count_before_payment: usize,
    pub utxo_count_after_payment: usize,
    pub cost_to_empty_at_long_term_feerate: f32,
    pub balance: u64,
    pub waste_score: Option<f32>
}

impl Serialize for SimulationEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let input_count = if !self.inputs.is_empty() {
            Some(self.inputs.len())
        } else { None };

        let mut state = serializer.serialize_struct("SimulationEntry", 13)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("amount", &self.amount)?;
        state.serialize_field("fee", &self.fee)?;
        state.serialize_field("target_feerate", &self.target_feerate)?;
        state.serialize_field("real_feerate", &self.real_feerate)?;
        state.serialize_field("algorithm", &self.algorithm)?;
        state.serialize_field("input_count", &input_count)?;
        state.serialize_field("negative_effective_valued_inputs", &self.negative_effective_valued_inputs)?;
        state.serialize_field("output_count", &self.output_count)?;
        state.serialize_field("change_amount", &self.change_amount)?;
        state.serialize_field("utxo_count_before_payment", &self.utxo_count_before_payment)?;
        state.serialize_field("utxo_count_after_payment", &self.utxo_count_after_payment)?;
        state.serialize_field("waste_score", &self.waste_score)?;
        state.end()
    }
}

#[derive(Debug)]
pub struct SimulationSummary {
    algorithm_frequencies: HashMap<String, u32>,
    pub scenario_file: String,
    pub current_balance: u64,
    pub current_utxo_set_count: usize,
    pub deposit_count: usize,
    pub inputs_spent_count: usize,
    pub withdraw_count: usize,
    pub negative_effective_valued_inputs_count: usize,
    pub created_change_outputs_count: usize,
    pub changeless_transaction_count: usize,
    pub min_change_value: u64,
    pub max_change_value: u64,
    pub total_fees: f32,
    pub mean_fees_per_withdraw: f32,
    pub cost_to_empty_at_long_term_feerate: f32,
    pub total_cost: f32,
    pub min_input_set_size: usize,
    pub max_input_set_size: usize,
    pub change_values: Vec<f32>,
    pub input_set_sizes: Vec<f32>,
    pub usage: String,
}

impl Default for SimulationSummary {
    fn default() -> SimulationSummary {
        SimulationSummary {
            algorithm_frequencies: <HashMap<String, u32>>::default(),
            scenario_file: String::default(),
            current_balance: u64::default(),
            current_utxo_set_count: usize::default(),
            deposit_count: usize::default(),
            inputs_spent_count: usize::default(),
            withdraw_count: usize::default(),
            negative_effective_valued_inputs_count: usize::default(),
            created_change_outputs_count: usize::default(),
            changeless_transaction_count: usize::default(),
            min_change_value: u64::MAX,
            max_change_value: u64::MIN,
            total_fees: f32::default(),
            mean_fees_per_withdraw: f32::default(),
            cost_to_empty_at_long_term_feerate: f32::default(),
            total_cost: f32::default(),
            min_input_set_size: usize::MAX,
            max_input_set_size: usize::MIN,
            change_values: <Vec<f32>>::default(),
            input_set_sizes: <Vec<f32>>::default(),
            usage: String::default(),
        }
    }
}

impl SimulationSummary {
    pub fn update(&mut self, simulation_entry: &SimulationEntry) -> Result<(), Box<dyn Error>> {
        if simulation_entry.algorithm != "failed" {
            self.withdraw_count += 1;
        }

        self.algorithm_frequencies.entry(simulation_entry.algorithm.clone()).and_modify(|e| *e += 1).or_insert(1);

        self.current_balance = simulation_entry.balance;
        self.current_utxo_set_count = simulation_entry.utxo_count_after_payment;
        self.cost_to_empty_at_long_term_feerate = simulation_entry.cost_to_empty_at_long_term_feerate;

        self.negative_effective_valued_inputs_count += if let Some(count) = simulation_entry.negative_effective_valued_inputs {
            count
        } else { 0 };

        self.total_fees += if let Some(fee) = simulation_entry.fee {
            fee as f32
        } else { 0.0 };

        if simulation_entry.change_amount.is_none() {
            self.changeless_transaction_count += 1;
        } else {
            let change_value = simulation_entry.change_amount
                .ok_or("Should never fail because we already check is not None.")?;
            self.change_values.push(change_value as f32);
            self.created_change_outputs_count += 1;
            self.min_change_value = cmp::min(self.min_change_value, change_value);
            self.max_change_value = cmp::max(self.max_change_value, change_value);
        };

        if !simulation_entry.inputs.is_empty() {
            self.input_set_sizes.push(simulation_entry.inputs.len() as f32);
            self.min_input_set_size = cmp::min(self.min_input_set_size, simulation_entry.inputs.len());
            self.max_input_set_size = cmp::max(self.max_input_set_size, simulation_entry.inputs.len());
        }

        Ok(())
    }
}

impl Serialize for SimulationSummary {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        let std_dev_of_change_value = if self.change_values.len() > 1 {
            Some(standard_deviation(&self.change_values, None))
        } else { None };

        let std_dev_of_input_set_size = if self.input_set_sizes.len() > 1 {
            Some(standard_deviation(&self.input_set_sizes, None))
        } else { None };

        let usage = self.algorithm_frequencies.iter().map(|(key, value)| format!("{}: {}", key, value)).collect::<Vec<_>>().join(","); 

        let mut state = serializer.serialize_struct("SimulationSummary", 22)?;
        state.serialize_field("scenario_file", &self.scenario_file)?;
        state.serialize_field("current_balance", &self.current_balance)?;
        state.serialize_field("current_utxo_set_count", &self.current_utxo_set_count)?;
        state.serialize_field("deposit_count", &self.deposit_count)?;
        state.serialize_field("inputs_spent_count", &self.input_set_sizes.iter().sum::<f32>())?;
        state.serialize_field("withdraw_count", &self.withdraw_count)?;
        state.serialize_field("negative_effective_valued_inputs_count", &self.negative_effective_valued_inputs_count)?;
        state.serialize_field("created_change_outputs_count", &self.created_change_outputs_count)?;
        state.serialize_field("changeless_transaction_count", &self.changeless_transaction_count)?;
        state.serialize_field("min_change_value", &self.min_change_value)?;
        state.serialize_field("max_change_value", &self.max_change_value)?;
        state.serialize_field("mean_change_value", &(mean(&self.change_values)))?;
        state.serialize_field("std_dev_of_change_value", &std_dev_of_change_value)?;
        state.serialize_field("total_fees", &self.total_fees)?;
        state.serialize_field("mean_fees_per_withdraw", &(self.total_fees / self.withdraw_count as f32))?;
        state.serialize_field("cost_to_empty_at_long_term_fee_rate", &self.cost_to_empty_at_long_term_feerate)?;
        state.serialize_field("total_cost", &(self.total_cost + self.cost_to_empty_at_long_term_feerate))?;
        state.serialize_field("min_input_set_size", &self.min_input_set_size)?;
        state.serialize_field("max_input_set_size", &self.max_input_set_size)?;
        state.serialize_field("mean_input_set_size", &(mean(&self.input_set_sizes)))?;
        state.serialize_field("std_dev_of_input_set_size", &std_dev_of_input_set_size)?;
        state.serialize_field("usage", &usage)?;
        state.end()
    }
}
