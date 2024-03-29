use std::{
    cmp,
    env,
    error::Error,
    collections::HashMap,
    ffi::OsString,
    fs::{ File, OpenOptions },
    fs,
    process,
};

use statistical::{ mean, standard_deviation };

use bitcoin::amount::{ Amount, Denomination };

use bdk_coin_select::{ Candidate, CoinSelector, FeeRate, Target, TargetFee, ChangePolicy, DrainWeights };
use bdk_coin_select::metrics::LowestFee;

const SEGWIT_V1_TXIN_WEIGHT: u32 = 68;
const SEGWIT_V1_TXOUT_WEIGHT: u32 = 31;

#[derive(Debug, serde::Deserialize, Clone)]
struct ScenarioEntry {
    amount: f64,
    fee_rate_per_kvb: f64,
}

#[derive(Debug, serde::Serialize)]
struct SimulationEntry {
    id: usize,
    amount: u64,
    fee: Option<i64>,
    target_feerate: f32,
    real_feerate: Option<f32>,
    algorithm: String,
    input_count: Option<usize>,
    negative_effective_valued_utxos: Option<usize>,
    output_count: Option<usize>,
    change_amount: Option<u64>,
    utxo_count_before_payment: usize,
    utxo_count_after_payment: usize,
    waste_score: Option<f32>
}

#[derive(Debug, serde::Serialize)]
struct SimulationSummary {
    scenario_file: String,
    current_balance: u64,
    current_utxo_set_count: usize,
    deposit_count: usize,
    input_spent_count: usize,
    withdraw_count: usize,
    negative_effective_valued_utxos_spent_count: usize,
    created_change_outputs_count: usize,
    changeless_transaction_count: usize,
    min_change_value: u64,
    max_change_value: u64,
    mean_change_value: f32,
    std_dev_of_change_value: f32,
    total_fees: f32,
    mean_fees_per_withdraw: f32,
    cost_to_empty_at_long_term_fee_rate: f32,
    total_cost: f32,
    min_input_size: usize,
    max_input_size: usize,
    mean_input_size: f32,
    std_dev_of_input_size: f32, 
    usage: String,
}

struct PendingPayment {
    amount: u64,
    weight: u32,
}

fn run() -> Result<(), Box<dyn Error>> {
    let input_path = get_arg(1)?.into_string().expect("First argument should be a valid string.");
    let output_path = get_arg(2)?.into_string().expect("Second argument should be a valid string.");

    let scenario_file = File::open(&input_path)?;
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(scenario_file);

    fs::create_dir_all(&output_path)?;

    let full_results_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .append(true)
        .open(format!("{}/full_results.csv", &output_path))?;
    let mut full_results_writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(full_results_file);

    let inputs_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .append(true)
        .open(format!("{}/inputs.csv", &output_path))?;

    let mut inputs_writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(inputs_file);

    let results_sample_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .append(true)
        .open(format!("{}/results.csv", &output_path))?;
    let mut results_sample_writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(results_sample_file);

    let utxos_file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .append(true)
        .open(format!("{}/utxos.csv", &output_path))?;
    let mut utxos_writer = csv::WriterBuilder::new()
        .has_headers(false)
        .from_writer(utxos_file);

    let drain_weights = DrainWeights { output_weight: SEGWIT_V1_TXIN_WEIGHT, spend_weight: SEGWIT_V1_TXOUT_WEIGHT };
    let dust_limit = 526;
    let long_term_feerate = FeeRate::from_sat_per_vb(10.0);

    let mut candidates: Vec<Candidate> = Vec::new();
    let mut payments: Vec<PendingPayment> = Vec::new();
    let mut change_values: Vec<f32> = Vec::new();
    let mut input_sizes: Vec<f32> = Vec::new();
    let mut algorithm_frequencies: HashMap<&str, u32> = HashMap::new();
    let mut withdraw_attempt: usize = 0;

    let mut simulation_summary = SimulationSummary {
        scenario_file: input_path.split('/').last().expect("There should be at least one element in path.").to_string(),
        current_balance: 0,
        current_utxo_set_count: 0,
        deposit_count: 0,
        input_spent_count: 0,
        withdraw_count: 0,
        negative_effective_valued_utxos_spent_count: 0,
        created_change_outputs_count: 0,
        changeless_transaction_count: 0,
        min_change_value: 0,
        max_change_value: 0,
        mean_change_value: 0.0,
        std_dev_of_change_value: 0.0,
        total_fees: 0.0,
        mean_fees_per_withdraw: 0.0,
        cost_to_empty_at_long_term_fee_rate: 0.0,
        total_cost: 0.0,
        min_input_size: 0,
        max_input_size: 0,
        mean_input_size: 0.0,
        std_dev_of_input_size: 0.0, 
        usage: String::from(""),
    };
    for result in reader.deserialize() {
        let record: ScenarioEntry = result?;

        if record.amount > 0.0 {
            simulation_summary.deposit_count += 1;
            candidates.push(Candidate {
                input_count: 1,
                weight: SEGWIT_V1_TXIN_WEIGHT,
                value: Amount::from_btc(record.amount)?.to_sat(),
                is_segwit: true
            });
            continue;
        }

        withdraw_attempt += 1;

        let selection_inputs = candidates.clone();
        let utxo_count_before_payment: usize = selection_inputs.len();

        payments.push(PendingPayment {
            amount: Amount::from_btc(-1.0 * record.amount)?.to_sat(),
            weight: SEGWIT_V1_TXOUT_WEIGHT
        });

        let mut coin_selector = CoinSelector::fund_outputs(&selection_inputs, payments.iter().map(|x| x.weight));

        let target = Target {
            fee: TargetFee::from_feerate(FeeRate::from_btc_per_kvb(record.fee_rate_per_kvb as f32)),
            value: payments.iter().map(|x| x.amount).sum::<u64>(),
        };

        let negative_effective_valued_utxos = candidates.iter().filter(|x| x.effective_value(target.fee.rate) < 0.0).count();

        let utxos = candidates.iter().map(|x| Amount::from_sat(x.value).to_string_in(Denomination::Satoshi)).collect::<Vec<String>>();
        let utxo_amounts = utxos.join(",");
        utxos_writer.serialize((withdraw_attempt, utxo_amounts))?;

        if withdraw_attempt != 0 && withdraw_attempt % 500 == 0 {
            simulation_summary.usage = algorithm_frequencies.iter().map(|(key, value)| format!("{}: {}", key, value)).collect::<Vec<_>>().join(",");

            simulation_summary.cost_to_empty_at_long_term_fee_rate = candidates.len() as f32 * SEGWIT_V1_TXIN_WEIGHT as f32 * long_term_feerate.as_sat_vb();

            simulation_summary.total_cost = simulation_summary.total_fees + simulation_summary.cost_to_empty_at_long_term_fee_rate;

            simulation_summary.mean_fees_per_withdraw = simulation_summary.total_fees / simulation_summary.withdraw_count as f32;

            simulation_summary.mean_change_value = mean(&change_values);

            simulation_summary.std_dev_of_change_value = if change_values.len() > 1 {
                standard_deviation(&change_values, None)
            } else { 0.0 };

            simulation_summary.mean_input_size = mean(&input_sizes);

            simulation_summary.std_dev_of_input_size = if input_sizes.len() > 1 {
                standard_deviation(&input_sizes, None)
            } else { 0.0 };

            simulation_summary.current_balance = candidates.iter().map(|x| x.value).sum::<u64>();

            simulation_summary.current_utxo_set_count = candidates.len();

            results_sample_writer.serialize(&simulation_summary)?;
        };

        if !coin_selector.is_selection_possible(target) {
            algorithm_frequencies.entry("failed").and_modify(|e| *e += 1).or_insert(1);
            full_results_writer.serialize(
                SimulationEntry {
                    id: withdraw_attempt,
                    amount: target.value,
                    target_feerate: target.fee.rate.as_sat_vb(),
                    negative_effective_valued_utxos: Some(negative_effective_valued_utxos),
                    utxo_count_after_payment: candidates.len(),
                    utxo_count_before_payment,
                    algorithm: String::from("failed"),
                    fee: None,
                    change_amount: None,
                    input_count: None,
                    output_count: None,
                    real_feerate: None,
                    waste_score: None,
                }
            )?;
            continue;
        }

        simulation_summary.withdraw_count += 1;

        // We use a change policy that introduces a change output if doing so reduces
        // the "waste" and that the change output's value is at least that of the
        // `dust_limit`.
        let change_policy = ChangePolicy::min_value_and_waste(
            drain_weights,
            dust_limit,
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

        let mut algorithm = String::from("bnb");

        // We run the branch and bound algorithm with a max round limit of 100,000.
        match coin_selector.run_bnb(metric, 100_000) {
            Err(err) => {
                println!("failed to find a solution: {}", err);
                // fall back to naive selection
                coin_selector.select_until_target_met(target).expect("a selection was impossible!");
                algorithm_frequencies.entry("select_sorted").and_modify(|e| *e += 1).or_insert(1);
                algorithm = String::from("select_sorted");
            }
            Ok(score) => {
                println!("we found a solution with score {}", score);
                algorithm_frequencies.entry("bnb").and_modify(|e| *e += 1).or_insert(1);
            }
        };

        let change = coin_selector.drain(target, change_policy);
        simulation_summary.total_fees += coin_selector.fee(target.value, change.value) as f32;

        let selection = coin_selector
            .apply_selection(&selection_inputs)
            .collect::<Vec<_>>();


        let inputs = coin_selector.selected().map(|x| Amount::from_sat(x.1.value).to_string_in(Denomination::Satoshi)).collect::<Vec<String>>();
        let input_amounts = inputs.join(",");
        inputs_writer.serialize((withdraw_attempt, input_amounts))?;

        candidates = coin_selector.unselected().map(|x| x.1).collect::<Vec<_>>();

        payments.clear();

        println!("we selected {} inputs", selection.len());
        println!("We are including a change output of {} value (0 means not change)", change.value);

        change_values.push(change.value as f32);

        candidates.push(Candidate {
            input_count: 1,
            weight: SEGWIT_V1_TXIN_WEIGHT,
            value: change.value,
            is_segwit: true
        });

        let vin_size = coin_selector.selected().len();

        simulation_summary.input_spent_count += vin_size;
        input_sizes.push(vin_size as f32);
        simulation_summary.negative_effective_valued_utxos_spent_count += coin_selector.selected().map(|x| x.1).filter(|x| x.effective_value(target.fee.rate) < 0.0).count();

        simulation_summary.created_change_outputs_count += if change.value == 0 { 0 } else { 1 };
        simulation_summary.changeless_transaction_count += if change.value == 0 { 1 } else { 0 };

        simulation_summary.min_change_value = cmp::min(simulation_summary.min_change_value, change.value);
        simulation_summary.max_change_value = cmp::min(simulation_summary.max_change_value, change.value);
        simulation_summary.min_input_size = cmp::min(simulation_summary.min_input_size, coin_selector.selected().len());
        simulation_summary.max_input_size = cmp::min(simulation_summary.max_input_size, coin_selector.selected().len());

        full_results_writer.serialize(
            SimulationEntry {
                id: withdraw_attempt,
                amount: target.value,
                fee: Some(coin_selector.fee(target.value, change.value)),
                real_feerate: Some(coin_selector.implied_feerate(target.value, change).expect("selection is finished").as_sat_vb()),
                target_feerate: target.fee.rate.as_sat_vb(),
                negative_effective_valued_utxos: Some(negative_effective_valued_utxos),
                algorithm,
                input_count: Some(selection.len()),
                output_count: Some(if change.value != 0 {
                    payments.len() + 1_usize
                } else {
                    payments.len()
                }),
                change_amount: Some(change.value),
                utxo_count_after_payment: candidates.len(),
                utxo_count_before_payment,
                waste_score: Some(coin_selector.waste(target, long_term_feerate, change, 1.0)),
            }
        )?;
    }
    utxos_writer.flush()?;
    inputs_writer.flush()?;
    full_results_writer.flush()?;
    results_sample_writer.flush()?;
    Ok(())
}


fn get_arg(arg_num: usize) -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(arg_num) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(arg) => Ok(arg),
    }
}


fn main() {
    if let Err(err) = run() {
        eprintln!("{}", err);
        process::exit(1);
    }
}
