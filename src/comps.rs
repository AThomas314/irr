use ndarray::prelude::*;
use log::{debug, info};

pub fn compute_irr(
    payments: Array1<f64>,
    disbursements: Array1<f64>,
    tol: Option<f64>,
) -> Result<f64, Box<dyn std::error::Error>> {
    let z: Array1<f64> = Array1::from_elem((1,), 0.0);
    let payments: Array1<f64> = ndarray::concatenate![Axis(0), z, payments];
    let disbursements: Array1<f64> = ndarray::concatenate![Axis(0), disbursements, z];
    let values = payments - disbursements;
    debug!("Cash flow values: {:?}", values);
    let guess = make_guess(&values)?;
    let irr = newton_raphson(guess, values, tol)?;
    Ok(irr)
}

fn make_guess(values: &Array1<f64>) -> Result<f64, Box<dyn std::error::Error>> {
    static _10: usize = 5;
    let rates: Array1<f64> = array![0.05, 0.06, 0.07, 0.08, 0.09, 0.10, 0.12, 0.15, 0.18, 0.20];
    let len: usize = values.len();
    let powers = Array1::from_iter((0..len).map(|x| x as f64));
    let base_rates: Array1<f64> = (&rates / 365.25) + 1.0;
    let base_2d: Array2<f64> = base_rates.insert_axis(Axis(1));
    let powers_2d: Array2<f64> = powers.insert_axis(Axis(0));
    let discounting_factors: Array2<f64> = 1.0
        / Array2::from_shape_fn((rates.len(), len), |(i, j)| {
            base_2d[[i, 0]].powf(powers_2d[[0, j]])
        });
    debug!("discounting_factors {:?}", discounting_factors);
    let abs_npvs: Array1<f64> = discounting_factors.dot(values).abs();
    debug!("abs_npvs {:?}", abs_npvs);

    let min_idx = abs_npvs
        .indexed_iter()
        .min_by(|(_idx_1, x), (_idx_2, y)| x.total_cmp(y));
    let min_idx: usize = match min_idx {
        Some((idx, _val)) => idx,
        None => _10,
    };

    match rates.get(min_idx) {
        Some(rate) => Ok(*rate),
        None => {
            debug!("Could not find rate, returning 0.10");
            Ok(0.10)
        }
    }
}

fn newton_raphson(
    mut guess: f64,
    values: Array1<f64>,
    tol: Option<f64>,
) -> Result<f64, Box<dyn std::error::Error>> {
    let tol = match tol {
        Some(t) => t.abs(),
        None => 1e-2,
    };
    let mut npv = tol.abs() * 2.0;
    debug!(
        "Starting Newton-Raphson with guess {:?} and initial value {:?}",
        guess, npv
    );
    let len: usize = values.len();
    let powers = Array1::from_iter((0..len).map(|x| x as i32));
    debug!("powers {:?}", powers);
    let ones = Array1::from_elem(len, 1.0);
    debug!("ones {:?}", ones);
    while npv.abs() > tol {
        let rate_factor = 1.0 + (guess / 365.25);
        let discounting_factors = Array1::from_shape_fn(len, |i| 1.0 / rate_factor.powi(i as i32));
        npv = discounting_factors.dot(&values);

        let derivative: f64 = values
            .iter()
            .zip(discounting_factors.iter())
            .enumerate()
            .map(|(i, (v, df))| -(i as f64) * (v * df / rate_factor) / 365.25)
            .sum();
        guess = guess - npv / derivative;
        println!("Guess: {:.6}, NPV: {:.6}", guess, npv);
    }
    Ok(guess)
}

