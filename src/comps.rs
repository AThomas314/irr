//This module contains the logic to compute the irr
use crate::errors::BorrowingError;
use log::debug;

pub fn compute_irr(
    payment_dates: &[i32],
    payments: &[f64],
    disbursements_dates: &[i32],
    disbursements: &[f64],
    ref_date: i32,
    guess: f64,
    _tol: Option<f64>,
) -> Result<(), BorrowingError> {
    debug!(
        "{:#?},{:#?},{:#?},{:#?}",
        payment_dates, payments, disbursements_dates, disbursements
    );
    // Ok(irr)
    Ok(())
}
