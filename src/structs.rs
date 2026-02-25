use crate::comps::{compute_arrays, compute_irr};
use crate::consts::*;
use crate::errors::BorrowingError;
use log::debug;
use polars::prelude::*;
use polars_arrow::array::{BinaryViewArrayGeneric, MutablePrimitiveArray, PrimitiveArray};

use std::ops::Range;
#[derive(Debug)]
pub struct Borrowings {
    //class definitionc
    ids: BinaryViewArrayGeneric<str>,             // len = num loans
    locations: BinaryViewArrayGeneric<str>,       // len = num loans
    payment_amendment_dates: PrimitiveArray<i32>, //len = lenghth of payments file
    disbursement_amendment_dates: PrimitiveArray<i32>, //len = lenghth of disbursements file
    capitalization_dates: PrimitiveArray<i32>,    //len = num loans
    interest_payments: PrimitiveArray<f64>,       //len = lenghth of payments file
    principal_payments: PrimitiveArray<f64>,      //len = lenghth of payments file
    total_payments: PrimitiveArray<f64>,          //len = lenghth of payments file
    date_payments: PrimitiveArray<i32>,           //len = lenghth of payments file
    date_disbursements: PrimitiveArray<i32>,      //len = lenghth of disbursements file
    standalone_disbursements: PrimitiveArray<f64>, //len = lenghth of disbursements file
    consol_disbursements: PrimitiveArray<f64>,    //len = lenghth of disbursements file
    payments_ranges: Vec<Range<usize>>,           // len = num loans
    disbursements_ranges: Vec<Range<usize>>,      // len = num loans
}

impl Borrowings {
    pub fn new(
        // __init__
        mut payments_df: DataFrame,
        mut disbursements_df: DataFrame,
    ) -> Result<Self, BorrowingError> {
        let payment_ids = unique_loan_ids(payments_df.clone());
        let disbursement_ids = unique_loan_ids(disbursements_df.clone());
        payments_df = semi_joins(payments_df, disbursement_ids.clone())?;
        disbursements_df = semi_joins(disbursements_df, payment_ids.clone())?;
        payments_df.rechunk_mut();
        disbursements_df.rechunk_mut();
        let unique_loans_df = disbursements_df
            .clone()
            .lazy()
            .group_by_stable([col(LOAN_ID)])
            .agg([col(LOCATION).first(), col(CAP_DATE_COL).first()])
            .collect()?;
        let ids: BinaryViewArrayGeneric<str> = extract_strs(&unique_loans_df, LOAN_ID)?;
        let capitalization_dates: PrimitiveArray<i32> =
            extract_i32(&unique_loans_df, CAP_DATE_COL)?;
        let locations: BinaryViewArrayGeneric<str> = extract_strs(&unique_loans_df, LOCATION)?;
        let interest_payments: PrimitiveArray<f64> = extractf64(&payments_df, INTEREST_PAID)?;
        let principal_payments: PrimitiveArray<f64> = extractf64(&payments_df, PRINCIPAL_COL)?;
        let total_payments: PrimitiveArray<f64> = extractf64(&payments_df, TOTAL_PAID)?;
        let date_payments: PrimitiveArray<i32> = extract_i32(&payments_df, DATE_COL)?;
        let payment_amendment_dates: PrimitiveArray<i32> =
            extract_i32(&payments_df, AMENDMENT_DATE_COL)?;
        let disbursement_amendment_dates: PrimitiveArray<i32> =
            extract_i32(&disbursements_df, AMENDMENT_DATE_COL)?;
        let date_disbursements: PrimitiveArray<i32> = extract_i32(&disbursements_df, DATE_COL)?;
        let standalone_disbursements: PrimitiveArray<f64> =
            extractf64(&disbursements_df, STANDALONE)?;
        let consol_disbursements: PrimitiveArray<f64> = extractf64(&disbursements_df, CONSOL)?;
        let payments_ranges = create_ranges(&payments_df, ids.len() + 2)?;
        let disbursements_ranges = create_ranges(&disbursements_df, ids.len() + 2)?;
        // debug!(" disbursements_ranges {:#?}", disbursements_ranges);
        // debug!(" payments_ranges {:#?}", payments_ranges);
        Ok(Self {
            ids,
            locations,
            payment_amendment_dates,
            disbursement_amendment_dates,
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

    pub fn process(self) -> Result<(), BorrowingError> {
        //Result<HashMap<String, DataFrame>, BorrowingError> {
        for i in 0..self.ids.len() {
            //
            //
            //Loan Level Data Begins
            //
            //
            let id = &self.ids.value(i);
            let location = &self.locations.value(i);
            let payments_ranges = &self.payments_ranges[i];
            let disbursements_ranges = &self.disbursements_ranges[i];
            let capitalization_date = &self.capitalization_dates.values()[i];
            //Loan Level Data Ends
            let payments_amendment_dates: &[i32] =
                &self.payment_amendment_dates.values()[payments_ranges.start..payments_ranges.end];
            let disbursements_amendment_dates: &[i32] = &self.disbursement_amendment_dates.values()
                [disbursements_ranges.start..disbursements_ranges.end];
            let disbursements_amendment_splits: Vec<Range<usize>> =
                get_amendment_splits(disbursements_amendment_dates)?;
            let payments_amendment_splits: Vec<Range<usize>> =
                get_amendment_splits(payments_amendment_dates)?;
            let standalone_disbursements = &self.standalone_disbursements.values()
                [disbursements_ranges.start..disbursements_ranges.end];
            let date_disbursements = &self.date_disbursements.values()
                [disbursements_ranges.start..disbursements_ranges.end];
            let date_payments =
                &self.date_payments.values()[payments_ranges.start..payments_ranges.end];
            let total_payments =
                &self.total_payments.values()[payments_ranges.start..payments_ranges.end];
            let principal_payments =
                &self.principal_payments.values()[payments_ranges.start..payments_ranges.end];
            let interest_payments =
                &self.interest_payments.values()[payments_ranges.start..payments_ranges.end];
            //
            //
            // "Amendment" 1
            //
            //
            let payments_slice = &payments_amendment_splits[0];
            let disbursements_slice = &disbursements_amendment_splits[0];
            let cur_total_payments = &total_payments[payments_slice.start..payments_slice.end];
            let cur_total_standalone_disbursements =
                &standalone_disbursements[disbursements_slice.start..disbursements_slice.end];
            let cur_payment_dates = &date_payments[payments_slice.start..payments_slice.end];
            let cur_disbursements_dates =
                &date_disbursements[disbursements_slice.start..disbursements_slice.end];
            //
            //
            // Use the data extracted for this amendment to compute the irr
            //
            //
            let irr = compute_irr(
                cur_payment_dates,
                cur_total_payments,
                cur_disbursements_dates,
                cur_total_standalone_disbursements,
                cur_disbursements_dates[0],
                0.1,
                0.00001,
            )?;
            debug!("irr {:#?}", irr);
            let capacity = (date_payments.last().unwrap() - date_disbursements[0] + 1) as usize;
            println!("capacity {:#?}", capacity);
            let mut total_op_bal: MutablePrimitiveArray<f64> =
                MutablePrimitiveArray::<f64>::with_capacity(capacity);
            let mut total_cl_bal: MutablePrimitiveArray<f64> =
                MutablePrimitiveArray::<f64>::with_capacity(capacity);
            let mut total_interest: MutablePrimitiveArray<f64> =
                MutablePrimitiveArray::<f64>::with_capacity(capacity);
            let mut total_payments_padded: MutablePrimitiveArray<f64> =
                MutablePrimitiveArray::<f64>::with_capacity(capacity);
            let mut total_disbursements_padded: MutablePrimitiveArray<f64> =
                MutablePrimitiveArray::<f64>::with_capacity(capacity);
            let mut dates: Series = Int32Chunked::from_iter_values(
                "date".into(),
                (0..capacity as i32).map(|x| date_disbursements[0] + x),
            )
            .into_series();
            let mut irrs: MutablePrimitiveArray<f64> =
                MutablePrimitiveArray::<f64>::with_capacity(capacity);

            total_op_bal.extend_constant(capacity, Some(0.0));
            total_cl_bal.extend_constant(capacity, Some(0.0));
            total_interest.extend_constant(capacity, Some(0.0));
            total_payments_padded.extend_constant(capacity, Some(0.0));
            total_disbursements_padded.extend_constant(capacity, Some(0.0));
            irrs.extend_constant(capacity, Some(0.0)); // Dangerous! Elements are now "uninitialized" junk values
            //OK Because we're filling it up next before use.
            if let Some(next_amendment) = payments_amendment_splits.get(1) {
                compute_arrays(
                    cur_payment_dates,
                    cur_total_payments,
                    cur_disbursements_dates,
                    cur_total_standalone_disbursements,
                    0.0,
                    cur_disbursements_dates[0],
                    payments_amendment_dates[next_amendment.start],
                    irr,
                    &mut total_op_bal.values_mut_slice(),
                    &mut total_cl_bal.values_mut_slice(),
                    &mut total_interest.values_mut_slice(),
                    &mut total_payments_padded.values_mut_slice(),
                    &mut total_disbursements_padded.values_mut_slice(),
                    &mut irrs.values_mut_slice(),
                    0,
                );
            } else {
                compute_arrays(
                    cur_payment_dates,
                    cur_total_payments,
                    cur_disbursements_dates,
                    cur_total_standalone_disbursements,
                    0.0,
                    cur_disbursements_dates[0],
                    *cur_payment_dates.last().unwrap(),
                    irr,
                    &mut total_op_bal.values_mut_slice(),
                    &mut total_cl_bal.values_mut_slice(),
                    &mut total_interest.values_mut_slice(),
                    &mut total_payments_padded.values_mut_slice(),
                    &mut total_disbursements_padded.values_mut_slice(),
                    &mut irrs.values_mut_slice(),
                    0,
                );
            }

            //
            //
            //Amendment 1 DONE!
            //
            //
;
        }
        Ok(())
    }
}
fn extract_strs(df: &DataFrame, col: &str) -> Result<BinaryViewArrayGeneric<str>, BorrowingError> {
    //extract the string columns of the dataframe as an Arc<str>
    Ok(df
        .column(col)?
        .str()?
        .downcast_iter()
        .next()
        .unwrap()
        .to_owned())
    // .into_no_null_iter() //Assumes no nulls for performance, throws an error if nulls found, we've dropped nulls earlier to prevent this
    // .map(|s| Arc::from(s))
    // .collect())
}
fn extract_i32(df: &DataFrame, col: &str) -> Result<PrimitiveArray<i32>, BorrowingError> {
    //extract the date columns from the dataframe as an i32
    Ok(df
        .column(col)?
        .cast(&DataType::Int32)?
        .i32()?
        .downcast_iter()
        .next()
        .unwrap()
        .to_owned())
}
fn extractf64(df: &DataFrame, col: &str) -> Result<PrimitiveArray<f64>, BorrowingError> {
    //extract the numeric columns from the dataframe as an f64
    Ok(df
        .column(col)?
        .cast(&DataType::Float64)?
        .f64()?
        .downcast_iter()
        .next()
        .unwrap()
        .to_owned())
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
            map.push(i + 1);
        }
    }
    map.push(ids.len());
    Ok(usize_to_range(map))
}
fn usize_to_range(usizes: Vec<usize>) -> Vec<Range<usize>> {
    usizes.windows(2).map(|w| w[0]..w[1]).collect()
}

fn get_amendment_splits(dates: &[i32]) -> Result<Vec<Range<usize>>, BorrowingError> {
    let count =
        dates.windows(2).filter(|w| w[0] != w[1]).count() + if dates.is_empty() { 0 } else { 1 };
    let mut v = Vec::with_capacity(count);
    v.push(0);
    for i in 0..dates.len() - 1 {
        if dates[i] != dates[i + 1] {
            v.push(i + 1);
        }
    }
    v.push(dates.len());
    Ok(usize_to_range(v))
}

fn unique_loan_ids(df: DataFrame) -> LazyFrame {
    df.lazy()
        .select([col(LOAN_ID)])
        .unique(None, UniqueKeepStrategy::Any)
}

fn semi_joins(df: DataFrame, other: LazyFrame) -> Result<DataFrame, BorrowingError> {
    Ok(df
        .lazy()
        .join(
            other,
            [col(LOAN_ID)],
            [col(LOAN_ID)],
            JoinArgs::new(JoinType::Semi),
        )
        .collect()?)
}
