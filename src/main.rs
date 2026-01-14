mod comps;
mod errors;
mod funcs;
use comps::compute_irr;
use funcs::*;
use log::{debug, info};
use ndarray::prelude::*;
use polars::prelude::*;
use std::collections::HashMap;

// use polars::prelude::*;
// use rayon::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let payments =
        read_csv(r"C:\Users\ashis\OneDrive\Desktop\rust\irr\payments 1116 final 1 1.csv".into());
    let disbursements =
        read_csv(r"C:\Users\ashis\OneDrive\Desktop\rust\irr\disbursements 1116 3.csv".into());
    let mut data = HashMap::new();
    data.insert("disbursements".to_string(), disbursements);
    data.insert("payments".to_string(), payments);
    let mut borrowings_data = process_dataframe(data)?;
    let disbursements = borrowings_data.remove("disbursements").unwrap();
    let payments = borrowings_data.remove("payments").unwrap();
    let preprocessed = split_borrowings(payments, disbursements);
    println!("{:#?}", preprocessed);
    Ok(())
}

// #[pyfunction]
// #[pymodule]
// fn irr(m: &Bound<'_, PyModule>) -> PyResult<()> {
//     m.add_function(wrap_pyfunction!(process_dataframe, m)?)?;
//     Ok(())
// }
pub struct Borrowing {
    pub id: String,
    pub location: String,
    // We store the raw processed data here
    pub payments_df: DataFrame,
    pub disbursements_df: DataFrame,
}

impl Borrowing {
    pub fn new(
        id: String,
        location: String,
        payments_df: DataFrame,
        disbursements_df: DataFrame,
    ) -> Self {
        Self {
            id,
            location,
            payments_df,
            disbursements_df,
        }
    }
}
