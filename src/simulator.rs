use crate::PaymentPolicy;
use crate::SEGWIT_V1_TXOUT_WEIGHT;
use crate::models::{ PendingPayment, ScenarioEntry, SimulationSummary };
use crate::selectors::TargetSelector;

use std::{
    error::Error,
    fs::{ File, OpenOptions },
    fs,
};

use csv::Writer;

use bitcoin::amount::{ Amount, Denomination };

struct SimulationRecorder<T: std::io::Write> {
    utxos_writer: Writer<T>,
    inputs_writer: Writer<T>,
    samples_writer: Writer<T>,
    results_writer: Writer<T>,
}

impl<T: std::io::Write> SimulationRecorder<T> {
    fn new(output_path: String) -> Result<SimulationRecorder<std::fs::File>, Box<dyn Error>> {
        fs::create_dir_all(&output_path)?;

        let full_results_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .append(true)
            .open(format!("{}/full_results.csv", &output_path))?;
        let results_writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_writer(full_results_file);

        let inputs_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .append(true)
            .open(format!("{}/inputs.csv", &output_path))?;
        let inputs_writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(inputs_file);

        let results_sample_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .append(true)
            .open(format!("{}/results.csv", &output_path))?;
        let samples_writer = csv::WriterBuilder::new()
            .has_headers(true)
            .from_writer(results_sample_file);

        let utxos_file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .append(true)
            .open(format!("{}/utxos.csv", &output_path))?;
        let utxos_writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(utxos_file);

        Ok(SimulationRecorder {
            utxos_writer,
            inputs_writer,
            samples_writer,
            results_writer,
        })
    }

    fn flush(&mut self) -> Result<(), Box<dyn Error>> {
        self.utxos_writer.flush()?;
        self.inputs_writer.flush()?;
        self.samples_writer.flush()?;
        self.results_writer.flush()?;
        
        Ok(())
    }
}

pub struct Simulation<'a> {
    pub payment_policy: PaymentPolicy,
    pub selector: &'a mut (dyn TargetSelector + 'a)
}

impl Simulation<'_> {
    pub fn run(&mut self, input_path: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
        let mut simulation_summary = SimulationSummary::default();
        simulation_summary.scenario_file = input_path.split('/').last().expect("There should be at least one element in path.").to_string();

        let scenario_file = File::open(input_path)?;
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(scenario_file);

        let mut payments: Vec<PendingPayment> = Vec::new();
        let mut withdraw_attempt: usize = 0;
        let mut simulation_recorder = <SimulationRecorder<std::fs::File>>::new(output_path.to_string())?;

        for result in reader.deserialize() {
            let record: ScenarioEntry = result?;

            if record.amount > 0.0 {
                simulation_summary.deposit_count += 1;
                self.selector.deposit(record)?;
                continue;
            }

            withdraw_attempt += 1;

            payments.push(PendingPayment {
                amount: Amount::from_btc(-1.0 * record.amount)?.to_sat(),
                weight: SEGWIT_V1_TXOUT_WEIGHT
            });

            let utxo_amounts = self.selector
                .values()
                .into_iter().map(|x| Amount::from_sat(x).to_string_in(Denomination::Satoshi))
                .collect::<Vec<String>>()
                .join(",");
            simulation_recorder.utxos_writer.serialize((withdraw_attempt, utxo_amounts))?;

            let mut simulation_entry = self.selector.withdraw(&payments, record.fee_rate_per_kvb);

            match self.payment_policy {
                PaymentPolicy::Drop => payments.clear(),
                PaymentPolicy::RollForward if simulation_entry.algorithm != "failed" => payments.clear(),
                _ => (),
            }

            simulation_entry.id = withdraw_attempt;

            simulation_summary.update(&simulation_entry)?;

            let input_amounts = simulation_entry.inputs
                .iter()
                .map(|x| Amount::from_sat(*x).to_string_in(Denomination::Satoshi))
                .collect::<Vec<String>>()
                .join(",");
            simulation_recorder.inputs_writer.serialize((withdraw_attempt, input_amounts))?;

            if withdraw_attempt != 0 && withdraw_attempt % 500 == 0 {
                simulation_recorder.samples_writer.serialize(&simulation_summary)?;
            };

            simulation_recorder.results_writer.serialize(simulation_entry)?;
        }
        
        simulation_recorder.flush()?;

        Ok(())
    }
}
