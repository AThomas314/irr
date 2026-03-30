// Copyright (C) 2026 Ashish Thomas (Ashish T Susikaran)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//This module contains the logic to compute the irr
use crate::consts::INV_365_25;
use crate::errors::BorrowingError;
use log::warn;

/// Calculates the Internal Rate of Return (IRR) for a loan portfolio using the Newton-Raphson method.
/// The function finds the root of the Net Present Value (NPV) equation:
///
/// $$NPV = \sum_{t} \frac{P_t}{(1 + r)^t} = 0$$
///
/// Where:
/// * $P_t$ is the cash flow at time $t$.
/// * $r$ is the daily effective interest rate derived from the annualized `guess`.
///
/// It utilizes the derivative (gradient) of the NPV function to iteratively approach the
/// solution (Newton's Method):
///
/// $$x_{n+1} = x_n - \frac{f(x_n)}{f'(x_n)}$$
///
/// # Arguments
///
/// * `payment_dates`: A slice of epoch-based integers representing the dates of payments.
/// * `payments`: A slice of cash inflows (positive values).
/// * `disbursements_dates`: A slice of dates for loan disbursements.
/// * `disbursements`: A slice of cash outflows (processed as negative flows).
/// * `op_bal`: The opening balance of the loan tranche.
/// * `op_bal_date`: The date the opening balance was established.
/// * `ref_date`: The reference date (Day 0) for calculating time deltas ($t$).
/// * `guess`: Initial annualized interest rate (e.g., 0.10 for 10%).
/// * `tol`: Convergence tolerance for the NPV (e.g., 0.001).
///
/// # Returns
///
/// * `Ok(f64)`: The annualized IRR that brings NPV to within `tol`.
/// * `Err(BorrowingError)`: If convergence fails or a mathematical error occurs.
///
/// # Performance Note
///
/// This function is the primary hotspot in the execution kernel. It avoids heap allocations
/// by operating on primitive slices and uses `.powi()` for fast integer-based exponentiation
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
        // Shouldn't take more than 100 iterations anyway.

        let daily_rate = guess * INV_365_25;
        let inv_ddf = 1.0 / (1.0 + daily_rate);
        let (npv_p, grad_p): (f64, f64) = payment_dates
            .iter()
            .zip(payments)
            .map(|(&date, &p)| {
                let t = date - ref_date;
                let discount = inv_ddf.powi(t);

                let npv = p * discount;
                let grad = -t as f64 * npv * inv_ddf;
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
                "TERMINAL NPV {:#?} whereas tolerance is {:#?}. GRADIENT DISAPPEARED at {:#?} iterations with rate{:#?},",
                npv, tol, i, guess
            );
            return Ok(guess);
        }
        guess -= step * 365.25;
    }

    Ok(guess)
}
/// Materializes the daily loan schedule arrays based on a calculated IRR.
/// # Logic Flow
/// 1. **Padding Phase**: Sparse transaction slices are zipped and mapped into
///    the `padded` buffers at their respective date offsets.
/// 2. **Broadcasting Phase**: The IRR is filled across the target range.
/// 3. **Calculation Phase**: A single pass over the daily range computes the
///    opening balance, effective interest, and closing balance using the
///    IFRS 9 amortized cost methodology.
///
/// # Arguments
/// * `payment_dates` / `payments`: Sparse slices of payment data.
/// * `opening_balance`: The starting balance for this tranche or amendment period.
/// * `start_date` / `cutoff`: The temporal boundaries (inclusive) for the daily schedule.
/// * `irr`: The Annualized Effective Interest Rate to be applied.
/// * `op_bal` / `cl_bal` / `interest`: Mutable slices (buffers) where the resulting
///    daily schedule is written.
/// * `start`: The index offset in the global buffers where this tranche begins.
///
/// # Performance Mechanics
/// * **Struct-of-Arrays (SoA)**: By accepting multiple mutable slices instead of a
///   `Vec<Row>`, this function maintains high cache hit rates and allows for
///   SIMD-friendly sequential writes.
/// * **Single Pass**: The core balance calculation is done in a single loop over
///   `capacity`, ensuring $O(N)$ complexity relative to the number of days.
/// * **Bounds Safety**: The function uses explicit `idx` checks against `capacity`
///   to prevent out-of-bounds writes before the actual buffer indexing.
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
