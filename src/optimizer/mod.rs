pub mod cobyla;

pub enum OptimizerResult {
    Failure,
    NumResult,
}

pub struct Optimizer {
    pub x_temp: Vec<f64>,
    pub grad_temp: Vec<f64>,
    pub grad_temp_0: Vec<f64>,
    pub last_result: OptimizerResult,
    pub last_optf: f64,
}
