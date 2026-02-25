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
    // debug!("payments dates first {:#?}", payment_dates.first());
    // debug!("payments first {:#?}", payments.first());
    // debug!(
    //     "disbursements dates first {:#?}",
    //     disbursements_dates.first()
    // );
    // debug!("disbursements first {:#?}", disbursements.first());
    // debug!("payments dates last {:#?}", payment_dates.last());
    // debug!("payments last {:#?}", payments.last());
    // debug!("disbursements dates last {:#?}", disbursements_dates.last());
    // debug!("disbursements last {:#?}", disbursements.last());
    for i in 0..=100 {
        let daily_rate = guess / 365.25;
        let inv_ddf = 1.0 / (1.0 + daily_rate);
        let (npv_p, grad_p): (f64, f64) = payment_dates
            .iter()
            .zip(payments)
            .map(|(&date, &p)| {
                let t = date - ref_date;
                let discount = inv_ddf.powi(t);
                let npv = p * discount;
                let grad = -t as f64 * p * discount * inv_ddf;
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
                let npv = d * -1.0 * discount;
                let grad = -t as f64 * d * -1.0 * discount * inv_ddf;
                // debug!("{:#?}; {:#?}; {:#?}", t, d, npv);
                (npv, grad)
            })
            .fold((0.0, 0.0), |acc, x| (acc.0 + x.0, acc.1 + x.1));
        let npv = npv_d + npv_p;
        let grad = grad_d + grad_p;
        let step = npv / grad;
        if npv.abs().le(&tol) {
            debug!(
                "TERMINAL NPV AT {:#?} ITERATIONS {:#?} with irr {:#?} ",
                i, npv, guess
            );
            return Ok(guess);
        }
        if step.abs() < f64::EPSILON {
            debug!(
                "TERMINAL NPV {:#?} . GRADIENT DISAPPEARED at {:#?} iterations ",
                npv, i
            );
            return Ok(guess);
        }
        guess -= step * 365.25;
    }
    println!("{:#?}", guess);
    debug!("100 iterations done");
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
    debug!(
        "opening_balance {:#?},start_date {:#?} , cutoff {:#?} , irr {:#?}",
        opening_balance, start_date, cutoff, irr
    );
    let capacity = cutoff as usize - start_date as usize + 1 as usize;
    for ((((&date, &amt), &int), &pri), &rates) in payment_dates
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
    let daily_irr = irr / 365.25;
    for i in 0..capacity {
        let d_amt = total_disbursements[start + i];
        let p_amt = total_paid[start + i];
        let int_amt = (opening_balance + d_amt) * daily_irr;
        let closing = opening_balance + int_amt + d_amt - p_amt;
        op_bal[start + i] = opening_balance;
        cl_bal[start + i] = closing;
        interest[start + i] = int_amt;
        // debug!("i=={:#?}", i);
        // debug!("d_amt {:#?}", d_amt);
        // debug!("p_amt {:#?}", p_amt);
        // debug!("irr {:#?}", daily_irr);
        // debug!("opening_balance {:#?}", opening_balance);
        // debug!("int_amt {:#?}", int_amt);
        // debug!("closing {:#?}", closing);
        // debug!("\n");

        // op_bal.push(opening_balance);
        // interest.push(int_amt);
        // cl_bal.push(closing);
        opening_balance = closing;
    }
    // println!(
    //     "{:#?},{:#?},{:#?},{:#?},{:#?}",
    //     op_bal, cl_bal, interest, payments_padded, disbursements_padded
    // );
}
