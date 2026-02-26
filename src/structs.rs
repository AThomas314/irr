use crate::comps::{compute_arrays, compute_irr};
use crate::consts::*;
use crate::errors::BorrowingError;
use log::debug;
use polars::prelude::*;
use polars_arrow::array::{BinaryViewArrayGeneric, MutablePrimitiveArray, PrimitiveArray};
use polars_arrow::pushable::Pushable;
use std::fs::File;
use std::ops::{Div, Mul, Range};
#[derive(Debug)]
pub struct Borrowings {
    //class definitionc
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
    consol_disbursements: PrimitiveArray<f64>,    //len = length of disbursements file
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
        let interest_rates: PrimitiveArray<f64> = extractf64(&payments_df, RATE_COL)?;
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
            interest_rates,
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
            let id: &str = &self.ids.value(i);
            let location: &str = &self.locations.value(i);
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
            let interest_rates =
                &self.interest_rates.values()[payments_ranges.start..payments_ranges.end];

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
                0.0,
                0,
                cur_disbursements_dates[0],
                0.1,
                0.001,
            )?;
            debug!("irr {:#?}", irr);
            let capacity = (date_payments.last().unwrap() - date_disbursements[0] + 1) as usize;

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
                    payments_amendment_dates[next_amendment.start],
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
                    *cur_payment_dates.last().unwrap(),
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
                    debug!("payments_slice {:#?}", payments_slice);
                    let amendment_date = payments_amendment_dates[payments_slice.start];
                    let disbursements_slice: Option<Range<usize>> = disbursements_amendment_splits
                        .iter()
                        .find(|split| disbursements_amendment_dates[split.start] == amendment_date)
                        .cloned();
                    debug!("disbursements_slice {:#?}", disbursements_slice);
                    let cur_total_payments =
                        &total_payments[payments_slice.start..payments_slice.end];
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
                    let opening_balance_date: i32 = opening_balance_date.value().try_extract()?;
                    let irr = compute_irr(
                        cur_payment_dates,
                        cur_total_payments,
                        cur_disbursements_dates,
                        cur_total_standalone_disbursements,
                        opening_balance,
                        opening_balance_date,
                        first_disb_date,
                        0.1,
                        0.001,
                    )?;
                }
            };
            let mut df = build_as_dataframe(
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
            )
            .unwrap();
            let file = File::create("output.csv").expect("could not create file");
            let _ = CsvWriter::new(file).finish(&mut df);
        }
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
    let id = Series::from_iter((0..op_bals.len()).into_iter().map(|_| id)).with_name(ID.into());
    let capitalization_date =
        Series::from_iter((0..op_bals.len()).into_iter().map(|_| capitalization_date))
            .with_name(CAP_DATE_COL.into());
    let location = Series::from_iter((0..op_bals.len()).into_iter().map(|_| location))
        .with_name(LOCATION.into());
    let mut df = DataFrame::new(
        op_bals.len(),
        vec![
            id.into_column(),
            location.into_column(),
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
            capitalization_date.into_column(),
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
    let month = col(DATE_COL).cast(DataType::Date).dt().strftime("%b-%Y");
    let month_shifted = col(MONTH).shift(lit(1)).fill_null(col(MONTH));
    let payments_shifted = col(PAYMENTS).shift(lit(1)).fill_null(lit(0.0));
    let irr_shifted = col(IRR).shift(lit(1)).fill_null(lit(0.0));
    let sum_cols = [DISBURSEMENTS, INT_PAID, EIR_INT, PAYMENTS, INT_CAP, INT_EXP];
    let mut sum_aggs = Vec::from_iter(
        sum_cols
            .iter()
            .map(|c: &&str| col(PlSmallStr::from_str(c)).sum()),
    );
    let first_cols = [FROM, OP_BAL, MONTH, LOCATION];
    let mut first_aggs = Vec::from_iter(
        first_cols
            .iter()
            .map(|c: &&str| col(PlSmallStr::from_str(c)).first()),
    );
    let last_cols = [TO, INT_RATE, IRR, CL_BAL, ID];
    let mut last_aggs = Vec::from_iter(
        last_cols
            .iter()
            .map(|c: &&str| col(PlSmallStr::from_str(c)).last()),
    );
    let mut count_aggs = vec![col(GROUP).count().alias(DAYS)];
    let mut aggs: Vec<Expr> =
        Vec::with_capacity(first_aggs.len() + count_aggs.len() + last_aggs.len() + sum_aggs.len());
    aggs.append(&mut last_aggs);
    aggs.append(&mut first_aggs);
    aggs.append(&mut count_aggs);
    aggs.append(&mut sum_aggs);
    let amort = col(EIR_INT) - col(INT_PAID);
    let monthly_amort = amort.sum().over([col(MONTH)]);
    let months_days = col(DAYS).sum().over([col(MONTH)]);
    let daily_amort = monthly_amort.div(months_days);
    let amort = daily_amort.mul(col(DAYS)).alias(AMORTIZATION);
    df = df
        .lazy()
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
        .group_by_stable([col(GROUP)])
        .agg(aggs)
        .with_columns([
            col(FROM).cast(DataType::Date).dt().strftime("%d-%b-%Y"),
            col(TO).cast(DataType::Date).dt().strftime("%d-%b-%Y"),
            when(col(RATE_COL).eq(lit(0.0)))
                .then(col(RATE_COL).shift(lit(-1)))
                .otherwise(col(RATE_COL)),
            amort,
        ])
        .collect()?;

    // println!("{:#?}", df);
    Ok(df)
}
fn extract_strs(df: &DataFrame, col: &str) -> Result<BinaryViewArrayGeneric<str>, BorrowingError> {
    Ok(df
        .column(col)?
        .str()?
        .downcast_iter()
        .next()
        .unwrap()
        .to_owned())
}
fn extract_i32(df: &DataFrame, col: &str) -> Result<PrimitiveArray<i32>, BorrowingError> {
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
