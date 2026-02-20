use crate::errors::BorrowingError;
use log::{debug, info};

pub fn compute_irr(
    payment_dates: &[i32],
    payments: &[f64],
    disbursements_dates: &[i32],
    disbursements: &[f64],
    tol: Option<f64>,
) -> Result<(), BorrowingError> {
    debug!(
        "{:#?},{:#?},{:#?},{:#?}",
        payment_dates, payments, disbursements_dates, disbursements
    );
    // Ok(irr)
    Ok(())
}

// fn make_guess(values: &[f64]) -> Result<f64, BorrowingError> {
//     static _10: usize = 5;
//     let rates: Vec<f64> = array![0.05, 0.06, 0.07, 0.08, 0.09, 0.10, 0.12, 0.15, 0.18, 0.20];
//     let len: usize = values.len();
//     let powers = Vec::from_iter((0..len).map(|x| x as f64));
//     let base_rates: Vec<f64> = (&rates / 365.25) + 1.0;
//     let base_2d: Array2<f64> = base_rates.insert_axis(Axis(1));
//     let powers_2d: Array2<f64> = powers.insert_axis(Axis(0));
//     let discounting_factors: Array2<f64> = 1.0
//         / Array2::from_shape_fn((rates.len(), len), |(i, j)| {
//             base_2d[[i, 0]].powf(powers_2d[[0, j]])
//         });
//     debug!("discounting_factors {:?}", discounting_factors);
//     let abs_npvs: Vec<f64> = discounting_factors.dot(values).abs();
//     debug!("abs_npvs {:?}", abs_npvs);

//     let min_idx = abs_npvs
//         .indexed_iter()
//         .min_by(|(_idx_1, x), (_idx_2, y)| x.total_cmp(y));
//     let min_idx: usize = match min_idx {
//         Some((idx, _val)) => idx,
//         None => _10,
//     };

//     match rates.get(min_idx) {
//         Some(rate) => Ok(*rate),
//         None => {
//             debug!("Could not find rate, returning 0.10");
//             Ok(0.10)
//         }
//     }
// }

// fn newton_raphson(
//     mut guess: f64,
//     values: Vec<f64>,
//     tol: Option<f64>,
// ) -> Result<f64, BorrowingError> {
//     const MAX_ITER: u8 = 20;
//     const DAYS: f64 = 365.25;
//     let tol = match tol {
//         Some(t) => t.abs(),
//         None => 1e-2,
//     };
//     let mut npv = tol.abs() * 2.0;
//     debug!(
//         "Starting Newton-Raphson with guess {:?} and initial value {:?}",
//         guess, npv
//     );
//     let len: usize = values.len();
//     let powers = Vec::from_iter((0..len).map(|x| x as i32));
//     debug!("powers {:?}", powers);
//     let mut i: u8 = 0;
//     while npv.abs() > tol && i < MAX_ITER {
//         let rate_factor = 1.0 + (guess / DAYS);
//         let discounting_factors = Vec::from_shape_fn(len, |i| 1.0 / rate_factor.powi(i as i32));
//         npv = discounting_factors.dot(&values);

//         let derivative: f64 = values
//             .iter()
//             .zip(discounting_factors.iter())
//             .enumerate()
//             .map(|(i, (v, df))| -(i as f64) * (v * df / rate_factor) / DAYS)
//             .sum();
//         guess = guess - npv / derivative;
//         debug!("Guess: {:.6}, NPV: {:.6}", guess, npv);
//         i += 1;
//     }
//     Ok(guess)
// }
