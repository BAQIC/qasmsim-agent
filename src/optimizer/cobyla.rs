#[derive(Debug)]
pub struct Cobyla {
    pub max_eval: usize,
    pub initial_parameters: Vec<f64>,
    pub bounds: Vec<(f64, f64)>,
    pub f_tol: f64,
}

impl Default for Cobyla {
    fn default() -> Self {
        Cobyla {
            max_eval: usize::MAX,
            initial_parameters: Vec::default(),
            bounds: Vec::default(),
            f_tol: 1e-6,
        }
    }
}

impl Cobyla {
    pub fn new(
        max_eval: usize,
        initial_parameters: Vec<f64>,
        bounds: Vec<(f64, f64)>,
        f_tol: f64,
    ) -> Self {
        Cobyla {
            max_eval,
            initial_parameters,
            bounds,
            f_tol,
        }
    }

    pub fn optimize<F: cobyla::Func<()>>(
        &self,
        opt_func: F,
    ) -> Result<(Vec<f64>, f64), cobyla::FailStatus> {
        let cons: Vec<&dyn cobyla::Func<()>> = vec![];
        match cobyla::minimize(
            opt_func,
            &self.initial_parameters,
            &self.bounds,
            &cons,
            (),
            self.max_eval,
            cobyla::RhoBeg::All(0.5),
            Some(cobyla::StopTols {
                ftol_rel: self.f_tol,
                ..cobyla::StopTols::default()
            }),
        ) {
            Ok((_, x_opt, y_opt)) => Ok((x_opt, y_opt)),
            Err(e) => Err(e.0),
        }
    }
}
