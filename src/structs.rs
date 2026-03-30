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

use crate::comps::{compute_arrays, compute_irr};
use crate::consts::*;
use crate::errors::BorrowingError;
use crate::funcs::*;
use clap::*;
use polars::prelude::*;
use polars_arrow::array::{
    BinaryViewArrayGeneric, MutableArray, MutablePrimitiveArray, PrimitiveArray,
};
use rayon::prelude::*;
use std::fs::File;
use std::ops::{Div, Mul, Range};
#[derive(Debug)]
pub struct Borrowings {
    //class definition
    ids: BinaryViewArrayGeneric<str>,             // len = num loans
    locations: BinaryViewArrayGeneric<str>,       // len = num loans
    payment_amendment_dates: PrimitiveArray<i32>, //len = length of payments file
    disbursement_amendment_dates: PrimitiveArray<i32>, //len = length of disbursements file
    capitalization_dates: PrimitiveArray<i32>,    //len = num loans
    interest_payments: PrimitiveArray<f64>,       //len = length of payments file
    principal_payments: PrimitiveArray<f64>,      //len = length of payments file
    total_payments: PrimitiveArray<f64>,          //len = length of payments file
    interest_rates: PrimitiveArray<f64>,          //len = length of payments file
    date_payments: PrimitiveArray<i32>,           //len = length of payments file
    date_disbursements: PrimitiveArray<i32>,      //len = length of disbursements file
    standalone_disbursements: PrimitiveArray<f64>, //len = length of disbursements file
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
        let ids: BinaryViewArrayGeneric<str> = extract_strs(&unique_loans_df, LOAN_ID)?; //extract into arrays
        let capitalization_dates: PrimitiveArray<i32> =
            extract_i32(&unique_loans_df, CAP_DATE_COL)?;
        let locations: BinaryViewArrayGeneric<str> = extract_strs(&unique_loans_df, LOCATION)?; //extract into arrays
        let interest_payments: PrimitiveArray<f64> = extractf64(&payments_df, INTEREST_PAID)?; //extract into arrays
        let interest_rates: PrimitiveArray<f64> = extractf64(&payments_df, RATE_COL)?; //extract into arrays
        let principal_payments: PrimitiveArray<f64> = extractf64(&payments_df, PRINCIPAL_COL)?; //extract into arrays
        let total_payments: PrimitiveArray<f64> = extractf64(&payments_df, TOTAL_PAID)?; //extract into arrays
        let date_payments: PrimitiveArray<i32> = extract_i32(&payments_df, DATE_COL)?; //extract into arrays
        let payment_amendment_dates: PrimitiveArray<i32> =
            extract_i32(&payments_df, AMENDMENT_DATE_COL)?; //extract into arrays
        let disbursement_amendment_dates: PrimitiveArray<i32> =
            extract_i32(&disbursements_df, AMENDMENT_DATE_COL)?; //extract into arrays
        let date_disbursements: PrimitiveArray<i32> = extract_i32(&disbursements_df, DATE_COL)?; //extract into arrays
        let standalone_disbursements: PrimitiveArray<f64> =
            extractf64(&disbursements_df, STANDALONE)?; //extract into arrays
        let payments_ranges = create_ranges(&payments_df, ids.len() + 2)?;
        let disbursements_ranges = create_ranges(&disbursements_df, ids.len() + 2)?;
        Ok(Self {
            ids,
            locations,
            payment_amendment_dates,
            disbursement_amendment_dates,
            capitalization_dates,
            interest_payments,
            principal_payments,
            total_payments,
            interest_rates,
            date_payments,
            date_disbursements,
            standalone_disbursements,
            payments_ranges,
            disbursements_ranges,
        }) //Create the Borrowings struct
    }

    pub fn process(&self) -> Result<(), BorrowingError> {
        let _ =
            (0..self.ids.len())
                .into_par_iter()
                .try_for_each(|i| -> Result<(), BorrowingError> {
                    //
                    //
                    //Loan Level Data Begins
                    //
                    //
                    let id: &str = &self.ids.value(i);
                    let location: &str = &self.locations.value(i);
                    let payments_ranges = &self.payments_ranges[i];
                    let disbursements_ranges = &self.disbursements_ranges[i];
                    let capitalization_date = &self.capitalization_dates.values()[i];
                    //Loan Level Data Ends
                    let payments_amendment_dates: &[i32] = &self.payment_amendment_dates.values()
                        [payments_ranges.start..payments_ranges.end]; //This Loans payments Amendment dates
                    let disbursements_amendment_dates: &[i32] = &self
                        .disbursement_amendment_dates
                        .values()[disbursements_ranges.start..disbursements_ranges.end]; //This Loans Disbursements Amendment dates
                    let disbursements_amendment_splits: Vec<Range<usize>> =
                        get_amendment_splits(disbursements_amendment_dates)?; //Using the disbursements amendments dates to get the ranges corresponding to each amendment
                    let payments_amendment_splits: Vec<Range<usize>> =
                        get_amendment_splits(payments_amendment_dates)?; //Using the payments amendments dates to get the ranges corresponding to each amendment

                    let standalone_disbursements = &self.standalone_disbursements.values()
                        [disbursements_ranges.start..disbursements_ranges.end]; //This loans standalone disbursements
                    let date_disbursements = &self.date_disbursements.values()
                        [disbursements_ranges.start..disbursements_ranges.end]; //This loans disbursements dates
                    let date_payments =
                        &self.date_payments.values()[payments_ranges.start..payments_ranges.end]; //This loans payment dates
                    let total_payments =
                        &self.total_payments.values()[payments_ranges.start..payments_ranges.end]; //This loans total payments
                    let principal_payments = &self.principal_payments.values()
                        [payments_ranges.start..payments_ranges.end]; //this loans principal payments
                    let interest_rates =
                        &self.interest_rates.values()[payments_ranges.start..payments_ranges.end]; //this loans interest rates

                    let interest_payments = &self.interest_payments.values()
                        [payments_ranges.start..payments_ranges.end]; //this loans interest payments
                    //
                    //
                    // "Amendment" 1
                    //
                    //
                    let payments_slice = &payments_amendment_splits[0]; //Get the first split
                    let disbursements_slice = &disbursements_amendment_splits[0]; //Get the first split
                    //
                    // Getting the values from the first split
                    //
                    let cur_total_payments =
                        &total_payments[payments_slice.start..payments_slice.end];
                    let cur_total_standalone_disbursements = &standalone_disbursements
                        [disbursements_slice.start..disbursements_slice.end];
                    let cur_payment_dates =
                        &date_payments[payments_slice.start..payments_slice.end];
                    let cur_disbursements_dates =
                        &date_disbursements[disbursements_slice.start..disbursements_slice.end];
                    //
                    // Got the values from the first split
                    //

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
                        0.0,
                        0,
                        cur_disbursements_dates[0],
                        interest_rates[0],
                        0.001,
                    )?;
                    let capacity =
                        (date_payments.last().unwrap() - date_disbursements[0] + 1) as usize; //Number of days between first disbursement and last payment of the amendment (Inclusiv of beginning and end)
                    // Allocating on the heap
                    let mut total_op_bal: MutablePrimitiveArray<f64> =
                        MutablePrimitiveArray::<f64>::with_capacity(capacity);
                    let mut total_cl_bal: MutablePrimitiveArray<f64> =
                        MutablePrimitiveArray::<f64>::with_capacity(capacity);
                    let mut total_interest: MutablePrimitiveArray<f64> =
                        MutablePrimitiveArray::<f64>::with_capacity(capacity);
                    let mut total_payments_padded: MutablePrimitiveArray<f64> =
                        MutablePrimitiveArray::<f64>::with_capacity(capacity);
                    let mut interest_payments_padded: MutablePrimitiveArray<f64> =
                        MutablePrimitiveArray::<f64>::with_capacity(capacity);
                    let mut principal_payments_padded: MutablePrimitiveArray<f64> =
                        MutablePrimitiveArray::<f64>::with_capacity(capacity);
                    let mut interest_rates_padded: MutablePrimitiveArray<f64> =
                        MutablePrimitiveArray::<f64>::with_capacity(capacity);
                    let mut total_disbursements_padded: MutablePrimitiveArray<f64> =
                        MutablePrimitiveArray::<f64>::with_capacity(capacity);
                    let dates: Series = Int32Chunked::from_iter_values(
                        DATE_COL.into(),
                        (0..capacity as i32).map(|x| date_disbursements[0] + x),
                    )
                    .into_series();
                    let mut irrs: MutablePrimitiveArray<f64> =
                        MutablePrimitiveArray::<f64>::with_capacity(capacity);
                    // Filling with zeroes
                    total_op_bal.extend_constant(capacity, Some(0.0));
                    total_cl_bal.extend_constant(capacity, Some(0.0));
                    total_interest.extend_constant(capacity, Some(0.0));
                    total_payments_padded.extend_constant(capacity, Some(0.0));
                    total_disbursements_padded.extend_constant(capacity, Some(0.0));
                    interest_payments_padded.extend_constant(capacity, Some(0.0));
                    principal_payments_padded.extend_constant(capacity, Some(0.0));
                    interest_rates_padded.extend_constant(capacity, Some(0.0));
                    irrs.extend_constant(capacity, Some(0.0));

                    if let Some(next_amendment) = payments_amendment_splits.get(1) {
                        compute_arrays(
                            cur_payment_dates,
                            cur_total_payments,
                            interest_payments,
                            principal_payments,
                            interest_rates,
                            cur_disbursements_dates,
                            cur_total_standalone_disbursements,
                            0.0,
                            cur_disbursements_dates[0],
                            payments_amendment_dates[next_amendment.start], //Cutoff date taken from here
                            irr,
                            &mut total_op_bal.values_mut_slice(),
                            &mut total_cl_bal.values_mut_slice(),
                            &mut total_interest.values_mut_slice(),
                            &mut total_payments_padded.values_mut_slice(),
                            &mut total_disbursements_padded.values_mut_slice(),
                            &mut irrs.values_mut_slice(),
                            &mut interest_payments_padded.values_mut_slice(),
                            &mut principal_payments_padded.values_mut_slice(),
                            &mut interest_rates_padded.values_mut_slice(),
                            0,
                        );
                    } else {
                        compute_arrays(
                            cur_payment_dates,
                            cur_total_payments,
                            interest_payments,
                            principal_payments,
                            interest_rates,
                            cur_disbursements_dates,
                            cur_total_standalone_disbursements,
                            0.0,
                            cur_disbursements_dates[0],
                            *cur_payment_dates.last().unwrap(), //Till the very last payment
                            irr,
                            &mut total_op_bal.values_mut_slice(),
                            &mut total_cl_bal.values_mut_slice(),
                            &mut total_interest.values_mut_slice(),
                            &mut total_payments_padded.values_mut_slice(),
                            &mut total_disbursements_padded.values_mut_slice(),
                            &mut irrs.values_mut_slice(),
                            &mut interest_payments_padded.values_mut_slice(),
                            &mut principal_payments_padded.values_mut_slice(),
                            &mut interest_rates_padded.values_mut_slice(),
                            0,
                        );
                    }
                    //
                    //
                    //Amendment 1 DONE!
                    //
                    //

                    //
                    //
                    //Subsequent Amendments Begin!
                    //
                    //
                    if payments_amendment_splits.len() > 1 {
                        for i in 1..payments_amendment_splits.len() {
                            let payments_slice = &payments_amendment_splits[i];
                            // debug!("payments_slice {:#?}", payments_slice);
                            let amendment_date = payments_amendment_dates[payments_slice.start];
                            let disbursements_slice: Option<Range<usize>> =
                                disbursements_amendment_splits
                                    .iter()
                                    .find(|split| {
                                        disbursements_amendment_dates[split.start] == amendment_date
                                    })
                                    .cloned();
                            // debug!("disbursements_slice {:#?}", disbursements_slice);
                            let cur_total_payments =
                                &total_payments[payments_slice.start..payments_slice.end];
                            let cur_interest_rates =
                                &interest_rates[payments_slice.start..payments_slice.end];
                            // debug!("payments_slice {:#?}", payments_slice);
                            let (cur_total_standalone_disbursements, cur_disbursements_dates) =
                                if let Some(slice) = disbursements_slice {
                                    (
                                        &standalone_disbursements[slice.start..slice.end],
                                        &date_disbursements[slice.start..slice.end],
                                    )
                                } else {
                                    (&[0.0][..], &[amendment_date][..])
                                };
                            let cur_payment_dates =
                                &date_payments[payments_slice.start..payments_slice.end];
                            let first_disb_date = date_disbursements[0];
                            let offset: usize = (amendment_date - first_disb_date) as usize;
                            let opening_balance = total_cl_bal.values()[offset];
                            let opening_balance_date = dates.slice(offset as i64, 1).first();
                            let opening_balance_date: i32 =
                                opening_balance_date.value().try_extract()?;
                            let irr = compute_irr(
                                cur_payment_dates,
                                cur_total_payments,
                                cur_disbursements_dates,
                                cur_total_standalone_disbursements,
                                opening_balance,
                                opening_balance_date,
                                first_disb_date,
                                cur_interest_rates[0],
                                0.001,
                            )?;
                            if let Some(next_amendment) = payments_amendment_splits.get(i + 1) {
                                // debug!("next_amendment {:#?}", next_amendment);
                                compute_arrays(
                                    cur_payment_dates,
                                    cur_total_payments,
                                    interest_payments,
                                    principal_payments,
                                    cur_interest_rates,
                                    cur_disbursements_dates,
                                    cur_total_standalone_disbursements,
                                    opening_balance,
                                    cur_disbursements_dates[0],
                                    payments_amendment_dates[next_amendment.start], //cutoff
                                    irr,
                                    &mut total_op_bal.values_mut_slice(),
                                    &mut total_cl_bal.values_mut_slice(),
                                    &mut total_interest.values_mut_slice(),
                                    &mut total_payments_padded.values_mut_slice(),
                                    &mut total_disbursements_padded.values_mut_slice(),
                                    &mut irrs.values_mut_slice(),
                                    &mut interest_payments_padded.values_mut_slice(),
                                    &mut principal_payments_padded.values_mut_slice(),
                                    &mut interest_rates_padded.values_mut_slice(),
                                    offset,
                                );
                            } else {
                                // debug!("next_amendment :NONE",);
                                compute_arrays(
                                    cur_payment_dates,
                                    cur_total_payments,
                                    interest_payments,
                                    principal_payments,
                                    cur_interest_rates,
                                    cur_disbursements_dates,
                                    cur_total_standalone_disbursements,
                                    opening_balance,
                                    cur_disbursements_dates[0],
                                    *cur_payment_dates.last().unwrap(), //cutoff
                                    irr,
                                    &mut total_op_bal.values_mut_slice(),
                                    &mut total_cl_bal.values_mut_slice(),
                                    &mut total_interest.values_mut_slice(),
                                    &mut total_payments_padded.values_mut_slice(),
                                    &mut total_disbursements_padded.values_mut_slice(),
                                    &mut irrs.values_mut_slice(),
                                    &mut interest_payments_padded.values_mut_slice(),
                                    &mut principal_payments_padded.values_mut_slice(),
                                    &mut interest_rates_padded.values_mut_slice(),
                                    offset,
                                );
                            }
                        }
                    };
                    let df = build_as_dataframe(
                        id,
                        location,
                        dates,
                        total_op_bal,
                        total_interest,
                        interest_payments_padded,
                        principal_payments_padded,
                        total_cl_bal,
                        total_payments_padded,
                        total_disbursements_padded,
                        interest_rates_padded,
                        irrs,
                        *capitalization_date,
                    );
                    if let Ok(mut df) = df {
                        let file = File::create("output.csv").expect("could not create file");
                        let _ = CsvWriter::new(file).finish(&mut df);
                    }
                    Ok(())
                });
        Ok(())
    }
}
fn build_as_dataframe(
    id: &str,
    location: &str,
    dates: Series,
    op_bals: MutablePrimitiveArray<f64>,
    effective_interest: MutablePrimitiveArray<f64>,
    interest_payments: MutablePrimitiveArray<f64>,
    principal_payments: MutablePrimitiveArray<f64>,
    cl_bals: MutablePrimitiveArray<f64>,
    payments: MutablePrimitiveArray<f64>,
    disbursements: MutablePrimitiveArray<f64>,
    interest_rates: MutablePrimitiveArray<f64>,
    irrs: MutablePrimitiveArray<f64>,
    capitalization_date: i32,
) -> Result<DataFrame, BorrowingError> {
    let mut df = DataFrame::new(
        op_bals.len(),
        vec![
            dates.into_column(),
            Series::from_array(OP_BAL.into(), PrimitiveArray::from(op_bals)).into_column(),
            Series::from_array(EIR_INT.into(), PrimitiveArray::from(effective_interest))
                .into_column(),
            Series::from_array(INT_PAID.into(), PrimitiveArray::from(interest_payments))
                .into_column(),
            Series::from_array(PRINC_PAID.into(), PrimitiveArray::from(principal_payments))
                .into_column(),
            Series::from_array(CL_BAL.into(), PrimitiveArray::from(cl_bals)).into_column(),
            Series::from_array(PAYMENTS.into(), PrimitiveArray::from(payments)).into_column(),
            Series::from_array(DISBURSEMENTS.into(), PrimitiveArray::from(disbursements))
                .into_column(),
            Series::from_array(INT_RATE.into(), PrimitiveArray::from(interest_rates)).into_column(),
            Series::from_array(IRR.into(), PrimitiveArray::from(irrs)).into_column(),
        ],
    )?;

    let int_cap = when(col(DATE_COL).lt(col(CAP_DATE_COL)))
        .then(col(EIR_INT))
        .otherwise(lit(0.0))
        .alias(INT_CAP);
    let int_exp = when(col(DATE_COL).gt_eq(col(CAP_DATE_COL)))
        .then(col(EIR_INT))
        .otherwise(lit(0.0))
        .alias(INT_EXP);
    let month = col(DATE_COL).cast(DataType::Date).dt().truncate(lit("1mo"));
    let month_shifted = col(MONTH).shift(lit(1)).fill_null(col(MONTH));
    let payments_shifted = col(PAYMENTS).shift(lit(1)).fill_null(lit(0.0));
    let irr_shifted = col(IRR).shift(lit(1)).fill_null(lit(0.0));
    let sum_cols = [DISBURSEMENTS, INT_PAID, EIR_INT, PAYMENTS, INT_CAP, INT_EXP];
    let first_cols = [FROM, OP_BAL, MONTH, LOCATION];
    let last_cols = [TO, INT_RATE, IRR, CL_BAL, ID];
    // let count_cols = [DATE_COL];
    let mut aggs: Vec<Expr> =
        Vec::with_capacity(last_cols.len() + first_cols.len() + sum_cols.len() + 1);
    aggs.extend(
        last_cols
            .iter()
            .map(|&c| col(PlSmallStr::from_str(c)).last()),
    );
    aggs.extend(
        first_cols
            .iter()
            .map(|&c| col(PlSmallStr::from_str(c)).first()),
    );
    aggs.extend(
        [GROUP]
            .iter()
            .map(|&c| col(PlSmallStr::from_str(c)).count().alias(DAYS)),
    );
    aggs.extend(sum_cols.iter().map(|&c| col(PlSmallStr::from_str(c)).sum()));
    let amort = col(EIR_INT) - col(INT_PAID);
    let monthly_amort = amort.sum().over([col(MONTH)]);
    let months_days = col(DAYS).sum().over([col(MONTH)]);
    let daily_amort = monthly_amort.div(months_days);
    let amort = daily_amort.mul(col(DAYS)).alias(AMORTIZATION);
    df = df
        .lazy()
        .with_columns(vec![
            lit(id).alias(PlSmallStr::from_str(ID)),
            lit(location).alias(PlSmallStr::from_str(LOCATION)),
            lit(capitalization_date).alias(PlSmallStr::from_str(CAP_DATE_COL)),
        ])
        .with_columns(vec![
            month.alias(MONTH),
            int_cap,
            int_exp,
            col(DATE_COL).alias(FROM),
            col(DATE_COL).alias(TO),
        ])
        .with_column(
            col(MONTH)
                .neq(month_shifted)
                .or(payments_shifted.neq(lit(0.0)))
                .or(col(DISBURSEMENTS).fill_null(0.0).neq(lit(0.0)))
                .or(col(IRR).neq(irr_shifted))
                .cast(DataType::Int16)
                .cum_sum(false)
                .alias(GROUP), // Split at each month end, disbursement, payment, or amendment date, whichever is earliest
        )
        .group_by([col(GROUP)])
        .agg(aggs)
        .sort([FROM], SortMultipleOptions::default())
        .with_columns([
            col(FROM).cast(DataType::Date).dt().strftime("%d-%b-%Y"),
            col(TO).cast(DataType::Date).dt().strftime("%d-%b-%Y"),
            when(col(RATE_COL).eq(lit(0.0)))
                .then(col(RATE_COL).shift(lit(-1)))
                .otherwise(col(RATE_COL)),
            amort,
        ])
        .collect()?;
    Ok(df)
}

#[derive(Parser)]
pub struct FilePaths {
    pub payments_path: String,
    pub disbursements_path: String,
}
