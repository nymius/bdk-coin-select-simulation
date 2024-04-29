use crate::selectors::TargetSelector;
use crate::models::{ PendingPayment, ScenarioEntry, SimulationEntry };

use pyo3::prelude::{Python, PyModule, PyObject, PyAnyMethods };
use pyo3::types::IntoPyDict;

use std::error::Error;
use std::collections::HashMap;


pub struct PythonCoinSelect {
    py_selector: PyObject,
}

impl PythonCoinSelect {
    pub fn new(long_term_feerate: f32, dust_limit: u64, input_drain_weight: u32, output_drain_weight: u32) -> Result<Self, Box<dyn Error>> {
        Python::with_gil(|py| {
            let code = include_str!("./python_coin_select.py");
            let python_coin_selector: PyObject = PyModule::from_code_bound(
                py,
                code,
                "python_coin_select.py",
                "python_coin_select"
            )?
            .getattr("PythonCoinSelector")?
            .into();

            Ok(PythonCoinSelect {
                py_selector: python_coin_selector.call1(py, (long_term_feerate, dust_limit, input_drain_weight, output_drain_weight))?,
            })
        })
    }
}


impl TargetSelector for PythonCoinSelect {
    fn values(&self) -> Vec<u64> {
        Python::with_gil(|py| {
            self.py_selector
                .bind(py)
                .call_method("values", (), None)
                .unwrap()
                .extract()
                .unwrap()
        })
    }

    fn deposit(&mut self, record: ScenarioEntry) -> Result<(), Box<dyn Error>> {
        let mut kwargs = HashMap::<&str, ScenarioEntry>::new();
        kwargs.insert("scenario_entry", record);
        Python::with_gil(|py| {
            self.py_selector
                .bind(py)
                .call_method("deposit", (), Some(&kwargs.into_py_dict_bound(py)))
                .unwrap();
        });
        Ok(())
    }

    fn withdraw(&mut self, payments: &[PendingPayment], fee_rate_per_kvb: f32) -> SimulationEntry {
        Python::with_gil(|py| {
            let rust_dict: HashMap<String, PyObject> = self.py_selector
                .bind(py)
                .call_method("withdraw", (payments.to_vec(), fee_rate_per_kvb), None)
                .unwrap()
                .extract()
                .unwrap();

            SimulationEntry {
                id: rust_dict.get("id").unwrap().extract(py).unwrap(),
                inputs: rust_dict.get("inputs").unwrap().extract(py).unwrap(),
                amount: rust_dict.get("amount").unwrap().extract(py).unwrap(),
                fee: rust_dict.get("fee").unwrap().extract(py).unwrap(),
                target_feerate: rust_dict.get("target_feerate").unwrap().extract(py).unwrap(),
                real_feerate: rust_dict.get("real_feerate").unwrap().extract(py).unwrap(),
                algorithm: rust_dict.get("algorithm").unwrap().extract(py).unwrap(),
                negative_effective_valued_inputs: rust_dict.get("negative_effective_valued_inputs").unwrap().extract(py).unwrap(),
                output_count: rust_dict.get("output_count").unwrap().extract(py).unwrap(),
                change_amount: rust_dict.get("change_amount").unwrap().extract(py).unwrap(),
                utxo_count_before_payment: rust_dict.get("utxo_count_before_payment").unwrap().extract(py).unwrap(),
                utxo_count_after_payment: rust_dict.get("utxo_count_after_payment").unwrap().extract(py).unwrap(),
                cost_to_empty_at_long_term_feerate: rust_dict.get("cost_to_empty_at_long_term_feerate").unwrap().extract(py).unwrap(),
                balance: rust_dict.get("balance").unwrap().extract(py).unwrap(),
                waste_score: rust_dict.get("waste_score").unwrap().extract(py).unwrap(),
            }
        })
    }
}
