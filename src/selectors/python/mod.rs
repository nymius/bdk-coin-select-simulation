pub mod bitcoin_coin_selection;

use std::collections::HashMap;

use crate::models::{ PendingPayment, ScenarioEntry, SimulationEntry };
use pyo3::prelude::{ Python, PyObject, ToPyObject, IntoPy, pyclass };
use pyo3::types::PyDict;

#[derive(Clone, serde::Deserialize)]
#[pyclass]
struct PySimulationEntry(SimulationEntry);

impl ToPyObject for ScenarioEntry {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        let mut py_obj: HashMap<_, _> = HashMap::new();
        py_obj.insert("amount".to_string(), self.amount.to_object(py));
        py_obj.insert("fee_rate_per_kvb".to_string(), self.fee_rate_per_kvb.to_object(py));

        py_obj.to_object(py)
    }
}

impl ToPyObject for PendingPayment {
    fn to_object(&self, py: Python<'_>) -> PyObject {
        let mut py_obj: HashMap<_, _> = HashMap::new();
        py_obj.insert("amount".to_string(), self.amount.to_object(py));
        py_obj.insert("weight".to_string(), self.weight.to_object(py));

        py_obj.to_object(py)
    }
}

impl IntoPy<PyObject> for PendingPayment {
    fn into_py(self, py: Python<'_>) -> PyObject {
        self.to_object(py)
    }
}

impl From<PyDict> for PySimulationEntry {
    fn from(dict: PyDict) -> Self {
        let rust_dict: HashMap<String, PyObject> = dict.extract().unwrap();
        Python::with_gil(|py| {
            PySimulationEntry(SimulationEntry {
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
            })
        })
    }
}
