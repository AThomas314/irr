use log::{debug, info};
use ndarray::prelude::*;
mod comps;
use comps::compute_irr;
// use polars::prelude::*;
// use rayon::prelude::*;
// static PRINCIPAL_COL: &str = "Principal Paid";
// static RATE_COL: &str = "Interest Rate";
// static LOAN_AMOUNT_COL: &str = "Loan amount";
// static CG_COL: &str = "CG Allocation";
// static CG_GST_COL: &str = "CG GST";
// static LOAN_EXPS_COL: &str = "Loan expenses";
// static DATE_COL: &str = "Date";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // let irr = compute_irr();
    #[cfg(debug_assertions)]
    {
        println!("DEBUG MODE ACTIVE");
        env_logger::init();
    }
    let disbursements = array![1000.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let payments = array![0.0, 200.0, 300.0, 400.0, 500.0, 600.0];
    let irr = compute_irr(payments, disbursements, Some(1e-2))?;
    info!("Computed IRR: {:.6}%", irr * 100.0);
    Ok(())
}

struct Borrowings{
    payments: Array1<f64>,
    disbursements: Array1<f64>,
    location: String,
    id: String,
}