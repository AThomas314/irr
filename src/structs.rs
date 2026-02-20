use crate::consts::*;
use crate::errors::BorrowingError;
use log::debug;
use polars::prelude::*;
use std::ops::Range;
#[derive(Debug)]
pub struct Borrowings {
    pub ids: Vec<Arc<str>>,
    pub locations: Vec<Arc<str>>,
    pub amendment_dates: Vec<i32>,
    pub capitalization_dates: Vec<i32>,
    pub interest_payments: Vec<f64>,
    pub principal_payments: Vec<f64>,
    pub total_payments: Vec<f64>,
    pub date_payments: Vec<i32>,
    pub date_disbursements: Vec<i32>,
    pub standalone_disbursements: Vec<f64>,
    pub consol_disbursements: Vec<f64>,
    pub payments_ranges: Vec<Range<usize>>,
    pub disbursements_ranges: Vec<Range<usize>>,
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
        let ids: Vec<Arc<str>> = extract_strs(&unique_loans_df, LOAN_ID)?;
        let capitalization_dates: Vec<i32> = extract_i32(&unique_loans_df, CAP_DATE_COL)?;
        let locations: Vec<Arc<str>> = extract_strs(&unique_loans_df, LOCATION)?;
        let interest_payments: Vec<f64> = extractf64(&payments_df, INTEREST_PAID)?;
        let principal_payments: Vec<f64> = extractf64(&payments_df, PRINCIPAL_COL)?;
        let total_payments: Vec<f64> = extractf64(&payments_df, TOTAL_PAID)?;
        let date_payments: Vec<i32> = extract_i32(&payments_df, DATE_COL)?;
        let amendment_dates: Vec<i32> = extract_i32(&payments_df, AMENDMENT_DATE_COL)?;
        let date_disbursements: Vec<i32> = extract_i32(&disbursements_df, DATE_COL)?;
        let standalone_disbursements: Vec<f64> = extractf64(&disbursements_df, STANDALONE)?;
        let consol_disbursements: Vec<f64> = extractf64(&disbursements_df, CONSOL)?;
        let payments_ranges = create_ranges(&payments_df, ids.len() + 2)?;
        let disbursements_ranges = create_ranges(&disbursements_df, ids.len() + 2)?;
        debug!(" disbursements_ranges {:#?}", disbursements_ranges);
        debug!(" payments_ranges {:#?}", payments_ranges);
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
            payments_ranges,
            disbursements_ranges,
        })
    }
}

fn extract_strs(df: &DataFrame, col: &str) -> Result<Vec<Arc<str>>, BorrowingError> {
    Ok(df
        .column(col)?
        .str()?
        .into_no_null_iter()
        .map(|s| Arc::from(s))
        .collect())
}
fn extract_i32(df: &DataFrame, col: &str) -> Result<Vec<i32>, BorrowingError> {
    Ok(df
        .column(col)?
        .cast(&DataType::Int32)?
        .i32()?
        .cont_slice()?
        .to_vec())
}
fn extractf64(df: &DataFrame, col: &str) -> Result<Vec<f64>, BorrowingError> {
    Ok(df
        .column(col)?
        .cast(&DataType::Float64)?
        .f64()?
        .cont_slice()?
        .to_vec())
}

fn create_ranges(df: &DataFrame, len: usize) -> Result<Vec<Range<usize>>, BorrowingError> {
    let ids: Vec<&str> = df
        .column(LOAN_ID)?
        .str()?
        .into_no_null_iter()
        .map(|s| s)
        .collect();
    let mut map = Vec::with_capacity(len);
    map.push(0);
    for i in 0..ids.len() - 1 {
        if ids[i] != ids[i + 1] {
            map.push(i);
        }
    }
    map.push(ids.len());
    Ok(usize_to_range(map))
}
fn usize_to_range(usizes: Vec<usize>) -> Vec<Range<usize>> {
    let mut ranges = Vec::with_capacity(usizes.len() - 1);
    for i in 0..usizes.len() - 1 {
        let r = Range {
            start: usizes[i],
            end: usizes[i + 1],
        };
        ranges.push(r);
    }
    ranges
}
