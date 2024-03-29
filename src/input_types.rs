#[derive(Debug, serde::Deserialize, Clone)]
pub struct ScenarioEntry {
    pub amount: f64,
    pub fee_rate_per_kvb: f64,
}

pub struct PendingPayment {
    pub amount: u64,
    pub weight: u32,
}
