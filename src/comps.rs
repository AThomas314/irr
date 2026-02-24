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
    debug!("payments dates first {:#?}", payment_dates.first());
    debug!("payments first {:#?}", payments.first());
    debug!(
        "disbursements dates first {:#?}",
        disbursements_dates.first()
    );
    debug!("disbursements first {:#?}", disbursements.first());
    debug!("payments dates last {:#?}", payment_dates.last());
    debug!("payments last {:#?}", payments.last());
    debug!("disbursements dates last {:#?}", disbursements_dates.last());
    debug!("disbursements last {:#?}", disbursements.last());
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
                debug!("{:#?}; {:#?}; {:#?}", t, p, npv);
                (npv, grad)
            })
            .fold((0.0, 0.0), |acc, x| (acc.0 + x.0, acc.1 + x.1));
        let (npv_d, grad_d): (f64, f64) = disbursements_dates
            .iter()
            .zip(disbursements)
            .map(|(&date, &d)| {
                let t = date - ref_date;
                let discount = inv_ddf.powi(t - 1); // using t-1 so that interest is applied on the installment from the day on which it is disbursed. use t for only the day after
                let npv = d * -1.0 * discount;
                let grad = -t as f64 * d * -1.0 * discount * inv_ddf;
                debug!("{:#?}; {:#?}; {:#?}", t, d, npv);
                (npv, grad)
            })
            .fold((0.0, 0.0), |acc, x| (acc.0 + x.0, acc.1 + x.1));
        let npv = npv_d + npv_p;
        let grad = grad_d + grad_p;
        let step = npv / grad;
        if npv.abs().le(&tol) {
            debug!("TERMINAL NPV AT {:#?} ITERATIONS {:#?}", i, npv);
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
    disbursement_dates: &[i32],
    disbursements: &[f64],
    mut opening_balance: f64,
    start_date: i32,
    cutoff: i32,
    irr: f64,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    debug!(
        "{:#?},{:#?},{:#?},{:#?}",
        opening_balance, start_date, cutoff, irr
    );
    let capacity = cutoff as usize - start_date as usize + 1 as usize;
    let mut op_bal: Vec<f64> = Vec::with_capacity(capacity);
    let mut cl_bal: Vec<f64> = Vec::with_capacity(capacity);
    let mut interest: Vec<f64> = Vec::with_capacity(capacity);
    let mut payments_padded: Vec<f64> = vec![0.0; capacity];
    let mut disbursements_padded: Vec<f64> = vec![0.0; capacity];

    op_bal.push(0.0);

    for (&date, &amt) in payment_dates.iter().zip(payments) {
        let idx = (date - start_date) as usize;
        if idx >= capacity {
            break;
        }
        payments_padded[idx] = amt;
    }

    for (&date, &amt) in disbursement_dates.iter().zip(disbursements) {
        let idx = (date - start_date) as usize;
        if idx >= capacity {
            break;
        }
        disbursements_padded[idx] = amt;
    }
    let daily_irr = irr / 365.25;
    for i in 0..capacity {
        let d_amt = disbursements_padded[i];
        let p_amt = payments_padded[i];
        let int_amt = (opening_balance + d_amt) * daily_irr;
        let closing = opening_balance + int_amt + d_amt - p_amt;

        debug!("i=={:#?}", i);
        debug!("d_amt {:#?}", d_amt);
        debug!("p_amt {:#?}", p_amt);
        debug!("irr {:#?}", daily_irr);
        debug!("opening_balance {:#?}", opening_balance);
        debug!("int_amt {:#?}", int_amt);
        debug!("closing {:#?}", closing);
        debug!("\n");

        op_bal.push(opening_balance);
        interest.push(int_amt);
        cl_bal.push(closing);
        opening_balance = closing;
    }
    // println!(
    //     "{:#?},{:#?},{:#?},{:#?},{:#?}",
    //     op_bal, cl_bal, interest, payments_padded, disbursements_padded
    // );
    (
        op_bal,
        cl_bal,
        interest,
        payments_padded,
        disbursements_padded,
    )
}
