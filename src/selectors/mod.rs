pub mod bdk;

use std::error::Error;
use crate::{ ScenarioEntry, SimulationEntry, PendingPayment };

pub trait TargetSelector {
    fn deposit(&mut self, deposit: ScenarioEntry) -> Result<(), Box<dyn Error>>;
    fn withdraw(&mut self, payments: &[PendingPayment], fee_rate_per_kvb: f32) -> SimulationEntry;
    fn values(&self) -> Vec<u64>;
}
