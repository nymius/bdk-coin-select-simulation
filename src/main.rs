mod models;
mod selectors;
mod simulator;

use crate::simulator::Simulation;
use crate::selectors::bdk::BdkCoinSelect;
use crate::selectors::rust_coinselect::RustCoinSelect;
use crate::models::{ PendingPayment, ScenarioEntry, SimulationEntry };

use std::{
    env,
    error::Error,
    ffi::OsString,
    process,
};

const SEGWIT_V1_TXIN_WEIGHT: u32 = 68;
const SEGWIT_V1_TXOUT_WEIGHT: u32 = 31;

#[derive(Default, Copy, Clone)]
enum PaymentPolicy {
    #[default]
    RollForward,
    Drop,
}

fn get_arg(arg_num: usize) -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(arg_num) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(arg) => Ok(arg),
    }
}

fn simulate() -> Result<(), Box<dyn Error>> {
    let input_path = get_arg(1)?.into_string().expect("First argument should be a valid string.");
    let output_path = get_arg(2)?.into_string().expect("Second argument should be a valid string.");

    let mut selector = RustCoinSelect::new(10.0, 526, SEGWIT_V1_TXIN_WEIGHT, SEGWIT_V1_TXOUT_WEIGHT);
    let mut simulation = Simulation {
        payment_policy: PaymentPolicy::Drop,
        selector: &mut selector
    };

    simulation.run(&input_path, &output_path)
}

fn main() {
    if let Err(err) = simulate() {
        eprintln!("{}", err);
        process::exit(1);
    }
}
