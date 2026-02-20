use crate::comps::compute_irr;
use crate::consts::*;
use crate::errors::BorrowingError;
use log::debug;
use polars::prelude::*;

#[derive(Debug)]
pub struct Borrowings {
    pub ids: Vec<Arc<String>>,
    pub locations: Vec<Arc<String>>,
    pub amendment_dates: Vec<i32>,
    pub capitalization_dates: Vec<i32>,
    pub interest_payments: Vec<f64>,
    pub principal_payments: Vec<f64>,
    pub total_payments: Vec<f64>,
    pub date_payments: Vec<i32>,
    pub date_disbursements: Vec<i32>,
    pub standalone_disbursements: Vec<f64>,
    pub consol_disbursements: Vec<f64>,
}

impl Borrowings {
    pub fn new(
        mut payments_df: DataFrame,
        mut disbursements_df: DataFrame,
    ) -> Result<Self, BorrowingError> {
        let payment_ids = payments_df
            .clone()
            .lazy()
            .select([col(LOAN_ID)])
            .unique(None, UniqueKeepStrategy::Any);
        let disbursement_ids = disbursements_df
            .clone()
            .lazy()
            .select([col(LOAN_ID)])
            .unique(None, UniqueKeepStrategy::Any);
        payments_df = payments_df
            .lazy()
            .join(
                disbursement_ids.clone(),
                [col(LOAN_ID)],
                [col(LOAN_ID)],
                JoinArgs::new(JoinType::Semi),
            )
            .collect()?;
        disbursements_df = disbursements_df
            .lazy()
            .join(
                payment_ids,
                [col(LOAN_ID)],
                [col(LOAN_ID)],
                JoinArgs::new(JoinType::Semi),
            )
            .collect()?;

        payments_df.rechunk_mut();
        disbursements_df.rechunk_mut();

        let unique_loans_df = disbursements_df
            .clone()
            .lazy()
            .group_by_stable([col(LOAN_ID)])
            .agg([col(LOCATION).first(), col(CAP_DATE_COL).first()])
            .collect()?;
        let ids: Vec<Arc<String>> = unique_loans_df
            .column(LOAN_ID)?
            .str()?
            .into_no_null_iter() // Skips null-checking branches
            .map(|s| Arc::new(s.to_string()))
            .collect();
        let capitalization_dates: Vec<i32> = unique_loans_df
            .column(CAP_DATE_COL)?
            .cast(&DataType::Int32)?
            .i32()?
            .cont_slice()
            .unwrap()
            .to_vec();
        let locations: Vec<Arc<String>> = unique_loans_df
            .column(LOCATION)?
            .str()?
            .into_no_null_iter()
            .map(|s| Arc::new(s.to_owned())) // Directly creates the String and wraps it
            .collect();

        let interest_payments: Vec<f64> = payments_df
            .column(INTEREST_PAID)?
            .f64()?
            .cont_slice()
            .unwrap()
            .to_vec();
        let principal_payments: Vec<f64> = payments_df
            .column(PRINCIPAL_COL)?
            .f64()
            .unwrap()
            .cont_slice()
            .unwrap()
            .to_vec();
        let total_payments: Vec<f64> = payments_df
            .column(TOTAL_PAID)?
            .f64()
            .unwrap()
            .cont_slice()
            .unwrap()
            .to_vec();
        let date_payments: Vec<i32> = payments_df
            .column(DATE_COL)?
            .cast(&DataType::Int32)?
            .i32()?
            .cont_slice()
            .unwrap()
            .to_vec();
        let amendment_dates: Vec<i32> = payments_df
            .column(AMENDMENT_DATE_COL)?
            .cast(&DataType::Int32)?
            .i32()?
            .cont_slice()
            .unwrap()
            .to_vec();

        let date_disbursements: Vec<i32> = disbursements_df
            .column(DATE_COL)?
            .cast(&DataType::Int32)?
            .i32()?
            .cont_slice()
            .unwrap()
            .to_vec();
        let standalone_disbursements: Vec<f64> = disbursements_df
            .column(STANDALONE)?
            .f64()?
            .cont_slice()
            .unwrap()
            .to_vec();
        let consol_disbursements: Vec<f64> = disbursements_df
            .column(CONSOL)?
            .f64()?
            .cont_slice()
            .unwrap()
            .to_vec();

        Ok(Self {
            ids,
            locations,
            amendment_dates,
            capitalization_dates,
            interest_payments,
            principal_payments,
            total_payments,
            date_payments,
            date_disbursements,
            standalone_disbursements,
            consol_disbursements,
        })
    }
}
