#[derive(Debug, serde::Serialize, Default)]
pub struct SimulationEntry {
    pub id: usize,
    pub amount: u64,
    pub fee: Option<i64>,
    pub target_feerate: f32,
    pub real_feerate: Option<f32>,
    pub algorithm: String,
    pub input_count: Option<usize>,
    pub negative_effective_valued_utxos: Option<usize>,
    pub output_count: Option<usize>,
    pub change_amount: Option<u64>,
    pub utxo_count_before_payment: usize,
    pub utxo_count_after_payment: usize,
    pub waste_score: Option<f32>
}

#[derive(Debug, serde::Serialize, Default)]
pub struct SimulationSummary {
    pub scenario_file: String,
    pub current_balance: u64,
    pub current_utxo_set_count: usize,
    pub deposit_count: usize,
    pub input_spent_count: usize,
    pub withdraw_count: usize,
    pub negative_effective_valued_utxos_spent_count: usize,
    pub created_change_outputs_count: usize,
    pub changeless_transaction_count: usize,
    pub min_change_value: u64,
    pub max_change_value: u64,
    pub mean_change_value: f32,
    pub std_dev_of_change_value: f32,
    pub total_fees: f32,
    pub mean_fees_per_withdraw: f32,
    pub cost_to_empty_at_long_term_fee_rate: f32,
    pub total_cost: f32,
    pub min_input_size: usize,
    pub max_input_size: usize,
    pub mean_input_size: f32,
    pub std_dev_of_input_size: f32,
    pub usage: String,
}
