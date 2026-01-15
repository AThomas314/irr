mod comps;
mod errors;
mod funcs;
use chrono::NaiveDate;
use comps::compute_irr;
use funcs::*;
use log::{debug, info};
use ndarray::prelude::*;
use polars::prelude::*;
use rayon::prelude::*;
use std::collections::{BTreeMap, HashMap};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let payments_future = read_payments(
        r"C:\Users\ashis\OneDrive\Desktop\rust\irr\payments 1116 final 1 1.csv".into(),
    );
    let disbursements_future = read_disbursements(
        r"C:\Users\ashis\OneDrive\Desktop\rust\irr\disbursements 1116 3.csv".into(),
    );
    let (payments, disbursements) = tokio::join!(payments_future, disbursements_future);
    let mut data = HashMap::new();
    data.insert(DISBURSEMENTS.to_string(), disbursements);
    data.insert(PAYMENTS.to_string(), payments);
    let disbursements = data.remove(DISBURSEMENTS).unwrap();
    let payments = data.remove(PAYMENTS).unwrap();
    debug!("{:#?}", payments);
    debug!("{:#?}", disbursements);
    let preprocessed = split_borrowings(payments, disbursements)?; // {"Loan ID" :{"Payments" :payments_df,"Disbursements":disbursements_df}}
    debug!("preprocessed {:#?}", preprocessed);
    let borrowings: Vec<Borrowing> = preprocessed
        .into_par_iter()
        .map(|(id, data)| {
            debug!("ID {:#?}: DATA {:#?}", id, data);
            Borrowing::new(id, data).unwrap()
        })
        .collect();
    info!("Borrowings {:#?}", borrowings);
    Ok(())
}

#[derive(Debug)]
pub struct Borrowing {
    pub id: String,
    pub location: String,
    pub payments: BTreeMap<NaiveDate, DataFrame>,
    pub disbursements: BTreeMap<NaiveDate, DataFrame>,
}

impl Borrowing {
    pub fn new(
        id: String,
        mut loan_data: HashMap<String, BTreeMap<NaiveDate, DataFrame>>, //{"Payments":payments_df,"Disbursements":disbursements_df}
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let disbursements = loan_data.remove(DISBURSEMENTS).unwrap();
        let payments = loan_data.remove(PAYMENTS).unwrap();
        let location = disbursements
            .last_key_value()
            .unwrap()
            .1
            .column(LOCATION)?
            .get(0)
            .unwrap()
            .to_string().replace("\"", "");
        Ok(Self {
            id,
            location,
            payments,
            disbursements,
        })
    }
}

// #[pyfunction]
// #[pymodule]
// fn irr(m: &Bound<'_, PyModule>) -> PyResult<()> {
//     m.add_function(wrap_pyfunction!(process_dataframe, m)?)?;
//     Ok(())
// }
