//This module contains the logic to compute the irr
use crate::consts::INV_365_25;
use crate::errors::BorrowingError;
use log::{debug, warn};
pub fn compute_irr(
    payment_dates: &[i32],
    payments: &[f64],
    disbursements_dates: &[i32],
    disbursements: &[f64],
    op_bal: f64,
    op_bal_date: i32,
    ref_date: i32,
    mut guess: f64,
    tol: f64,
) -> Result<f64, BorrowingError> {
    for i in 0..100 {
        // Shouldn't take more than 100 iterations.

        /// Uses the Newton-Raphson method find the IRR
        let daily_rate = guess * INV_365_25;
        let inv_ddf = 1.0 / (1.0 + daily_rate);
        let (npv_p, grad_p): (f64, f64) = payment_dates
            .iter()
            .zip(payments)
            .map(|(&date, &p)| {
                let t = date - ref_date;
                let discount = inv_ddf.powi(t);
                // let discount = pow(inv_ddf, t as f64);
                let npv = p * discount;
                let grad = -t as f64 * npv * inv_ddf;
                // debug!("{:#?}; {:#?}; {:#?}", t, p, npv);
                (npv, grad)
            })
            .fold((0.0, 0.0), |acc, x| (acc.0 + x.0, acc.1 + x.1));
        let (npv_d, grad_d): (f64, f64) = disbursements_dates
            .iter()
            .zip(disbursements)
            .map(|(&date, &d)| {
                let t = date - ref_date - 1;
                let discount = inv_ddf.powi(t);
                // let discount = pow(inv_ddf, t as f64);
                let npv = d * -1.0 * discount;
                let grad = -t as f64 * npv * inv_ddf;
                // debug!("{:#?}; {:#?}; {:#?}", t, d, npv);
                (npv, grad)
            })
            .fold((0.0, 0.0), |acc, x| (acc.0 + x.0, acc.1 + x.1));
        let (disc_ob, grad_ob) = {
            let t = op_bal_date - ref_date - 1;
            let discount = inv_ddf.powi(t);
            // let discount = pow(inv_ddf, t as f64);
            let disc_ob = op_bal * -1.0 * discount;
            let grad_ob = -t as f64 * disc_ob * inv_ddf;
            (disc_ob, grad_ob)
        };
        let npv = npv_d + npv_p + disc_ob;
        let grad = grad_d + grad_p + grad_ob;
        let step = npv / grad;
        if npv.abs().le(&tol) {
            return Ok(guess);
        }
        if step.abs() < f64::EPSILON {
            warn!(
                "TERMINAL NPV {:#?} . GRADIENT DISAPPEARED at {:#?} iterations with rate{:#?}",
                npv, i, guess
            );
            return Ok(guess);
        }
        guess -= step * 365.25;
    }

    Ok(guess)
}
pub fn compute_arrays(
    payment_dates: &[i32],
    payments: &[f64],
    interest_payments: &[f64],
    principal_payments: &[f64],
    interest_rates: &[f64],
    disbursement_dates: &[i32],
    disbursements: &[f64],
    mut opening_balance: f64,
    start_date: i32,
    cutoff: i32,
    irr: f64,
    op_bal: &mut [f64],
    cl_bal: &mut [f64],
    interest: &mut [f64],
    total_paid: &mut [f64],
    total_disbursements: &mut [f64],
    irrs: &mut [f64],
    interest_payments_padded: &mut [f64],
    principal_payments_padded: &mut [f64],
    interest_rates_padded: &mut [f64],
    start: usize,
) {
    let capacity = cutoff as usize - start_date as usize + 1 as usize;
    for ((((&date, &amt), &int), &pri), &rates) in payment_dates // ONLY WORKS BECAUSE THE SLICES HAVE THE SAME LENGTH
        .iter()
        .zip(payments)
        .zip(interest_payments)
        .zip(principal_payments)
        .zip(interest_rates)
    {
        let idx = (date - start_date) as usize;
        if idx >= capacity {
            break;
        }
        total_paid[start + idx] = amt;
        interest_payments_padded[start + idx] = int;
        principal_payments_padded[start + idx] = pri;
        interest_rates_padded[start + idx] = rates;
    }

    for (&date, &amt) in disbursement_dates.iter().zip(disbursements) {
        let idx = (date - start_date) as usize;
        if idx >= capacity {
            break;
        }
        total_disbursements[start + idx] = amt;
    }
    irrs[start..start + capacity].fill(irr);
    let daily_irr = irr * INV_365_25;
    for i in 0..capacity {
        let d_amt = total_disbursements[start + i];
        let p_amt = total_paid[start + i];
        let base = opening_balance + d_amt;
        let int_amt = base * daily_irr;
        let closing = base + int_amt - p_amt;
        op_bal[start + i] = opening_balance;
        cl_bal[start + i] = closing;
        interest[start + i] = int_amt;
        opening_balance = closing;
    }
}
