//This module contains the logic to compute the irr
use crate::errors::BorrowingError;
use log::debug;

pub fn compute_irr(
    payment_dates: &[i32],
    payments: &[f64],
    disbursements_dates: &[i32],
    disbursements: &[f64],
    ref_date: i32,
    mut guess: f64,
    tol: f64,
) -> Result<f64, BorrowingError> {
    for _ in 0..=100 {
        let daily_rate = guess / 365.25;
        let inv_ddf = 1.0 / (1.0 + daily_rate);
        let (npv_p, grad_p): (f64, f64) = payment_dates
            .iter()
            .zip(payments)
            .map(|(&date, &p)| {
                let t = (date - ref_date) as f64;
                let discount = inv_ddf.powi(date - ref_date);
                let npv = p * discount;
                let grad = -t * p * discount * inv_ddf;
                (npv, grad)
            })
            .fold((0.0, 0.0), |acc, x| (acc.0 + x.0, acc.1 + x.1));
        let (npv_d, grad_d): (f64, f64) = disbursements_dates
            .iter()
            .zip(disbursements)
            .map(|(&date, &d)| {
                let t = (date - ref_date) as f64;
                let discount = inv_ddf.powi(date - ref_date);
                let npv = d * -1.0 * discount;
                let grad = -t * d * -1.0 * discount * inv_ddf;
                (npv, grad)
            })
            .fold((0.0, 0.0), |acc, x| (acc.0 + x.0, acc.1 + x.1));
        let npv = npv_d + npv_p;
        let grad = grad_d + grad_p;
        let step = npv / grad;
        if npv.abs().le(&tol) {
            return Ok(guess);
        }
        if step.abs() < f64::EPSILON {
            return Ok(guess);
        }
        guess -= step * 365.25;
    }
    Ok(guess)
}
