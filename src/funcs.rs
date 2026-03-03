use crate::{consts::*, errors::BorrowingError};
use polars::prelude::*;
use polars_arrow::array::{BinaryViewArrayGeneric, PrimitiveArray};

use std::ops::Range;
use std::usize;
pub fn read_disbursements(path: &str) -> Result<DataFrame, BorrowingError> {
    let schema = Schema::from_iter(vec![
        Field::new(PlSmallStr::from_str(DATE_COL), DataType::String),
        Field::new(PlSmallStr::from_str(LOAN_AMOUNT_COL), DataType::String),
        Field::new(PlSmallStr::from_str(CG_COL), DataType::String),
        Field::new(PlSmallStr::from_str(CG_GST_COL), DataType::String),
        Field::new(PlSmallStr::from_str(LOAN_EXPS_COL), DataType::String),
        Field::new(PlSmallStr::from_str(LOAN_ID), DataType::String),
        Field::new(PlSmallStr::from_str(AMENDMENT_DATE_COL), DataType::String),
        Field::new(PlSmallStr::from_str(LOCATION), DataType::String),
        Field::new(PlSmallStr::from_str(CAP_DATE_COL), DataType::String),
    ]); //Read all required columns as String
    const FLOAT_COLS: [&str; 4] = [LOAN_AMOUNT_COL, CG_COL, CG_GST_COL, LOAN_EXPS_COL]; //Defining the columns as an array of consts for performance and easy iteration
    const DATE_COLS: [&str; 3] = [DATE_COL, AMENDMENT_DATE_COL, CAP_DATE_COL];

    const SELECTOR: [&str; 7] = [
        DATE_COL,
        AMENDMENT_DATE_COL,
        CAP_DATE_COL,
        STANDALONE,
        CONSOL,
        LOCATION,
        LOAN_ID,
    ];
    let mut expressions: Vec<Expr> = Vec::with_capacity(7); //creating a new empty Vec with the capacity predefined to avoid allocations

    for c in FLOAT_COLS {
        expressions.push(
            col(c)
                .str()
                .replace_all(lit(","), lit(""), false)
                .cast(DataType::Float64)
                .alias(c),
        );
    }
    for c in DATE_COLS {
        expressions.push(
            col(c)
                .str()
                .to_date(StrptimeOptions {
                    format: Some("%d-%m-%Y".into()),
                    strict: true,
                    ..Default::default()
                })
                .alias(c),
        );
    }
    let mut comps = Vec::with_capacity(2);
    comps.push(
        (col(LOAN_AMOUNT_COL) - col(CG_COL) - col(CG_GST_COL) - col(LOAN_EXPS_COL)).alias("Consol"),
    );
    comps.push((col(LOAN_AMOUNT_COL) - col(LOAN_EXPS_COL)).alias(STANDALONE));

    let df: LazyFrame = LazyCsvReader::new(PlRefPath::from(path))
        .with_schema(Some(Arc::from(schema)))
        .finish()?
        .with_columns(expressions)
        .drop_nulls(None)
        .with_columns(comps)
        .select(SELECTOR.map(col)); //creating the lazyframe and defining the operations thereon

    Ok(df.collect()?) //returning a Result containing the materialized dataframe
}

pub fn read_payments(path: &str) -> Result<DataFrame, BorrowingError> {
    //Follows the same logic as the read_disbursements function, hence have not commented
    let schema = Schema::from_iter(vec![
        Field::new(PlSmallStr::from_str(DATE_COL), DataType::String),
        Field::new(PlSmallStr::from_str(RATE_COL), DataType::String),
        Field::new(PlSmallStr::from_str(INTEREST_PAID), DataType::String),
        Field::new(PlSmallStr::from_str(PRINCIPAL_COL), DataType::String),
        Field::new(PlSmallStr::from_str(LOAN_ID), DataType::String),
        Field::new(PlSmallStr::from_str(AMENDMENT_DATE_COL), DataType::String),
    ]);
    const FLOAT_COLS: [&str; 3] = [INTEREST_PAID, RATE_COL, PRINCIPAL_COL];
    const DATE_COLS: [&str; 2] = [DATE_COL, AMENDMENT_DATE_COL];

    const SELECTOR: [&str; 7] = [
        INTEREST_PAID,
        RATE_COL,
        PRINCIPAL_COL,
        DATE_COL,
        AMENDMENT_DATE_COL,
        LOAN_ID,
        TOTAL_PAID,
    ];
    let mut expressions: Vec<Expr> =
        Vec::with_capacity(FLOAT_COLS.len() + DATE_COLS.len() as usize);

    for c in FLOAT_COLS {
        expressions.push(
            col(c)
                .str()
                .replace_all(lit(","), lit(""), false)
                .cast(DataType::Float64)
                .alias(c),
        );
    }
    for c in DATE_COLS {
        expressions.push(
            col(c)
                .str()
                .to_date(StrptimeOptions {
                    format: Some("%d-%m-%Y".into()),
                    strict: true,
                    ..Default::default()
                })
                .alias(c),
        );
    }
    let df: LazyFrame = LazyCsvReader::new(PlRefPath::from(path))
        .with_schema(Some(Arc::from(schema)))
        .finish()?
        .with_columns(expressions)
        .with_column((col(INTEREST_PAID) + col(PRINCIPAL_COL)).alias(TOTAL_PAID))
        .drop_nulls(None)
        .select(SELECTOR.map(col));

    Ok(df.collect()?)
}
pub fn extract_strs(
    df: &DataFrame,
    col: &str,
) -> Result<BinaryViewArrayGeneric<str>, BorrowingError> {
    Ok(df
        .column(col)?
        .str()?
        .downcast_iter()
        .next()
        .unwrap()
        .to_owned())
}
pub fn extract_i32(df: &DataFrame, col: &str) -> Result<PrimitiveArray<i32>, BorrowingError> {
    Ok(df
        .column(col)?
        .cast(&DataType::Int32)?
        .i32()?
        .downcast_iter()
        .next()
        .unwrap()
        .to_owned())
}
pub fn extractf64(df: &DataFrame, col: &str) -> Result<PrimitiveArray<f64>, BorrowingError> {
    Ok(df
        .column(col)?
        .cast(&DataType::Float64)?
        .f64()?
        .downcast_iter()
        .next()
        .unwrap()
        .to_owned())
}

pub fn create_ranges(df: &DataFrame, len: usize) -> Result<Vec<Range<usize>>, BorrowingError> {
    // Creates from the DataFrame a Vec of Ranges, this splits the payments df and disbursements df loanwise
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
pub fn usize_to_range(usizes: Vec<usize>) -> Vec<Range<usize>> {
    //Converts and ordered vec of usizes into a Vec of Ranges of Usizes
    usizes.windows(2).map(|w| w[0]..w[1]).collect()
}

pub fn get_amendment_splits(dates: &[i32]) -> Result<Vec<Range<usize>>, BorrowingError> {
    // Into this function we provide the slice of amendment dates from each dataframe, and it provides us the ranges of each amendment,
    //  which we use to slice the data for amendmentwise processing
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

pub fn unique_loan_ids(df: DataFrame) -> LazyFrame {
    df.lazy()
        .select([col(LOAN_ID)])
        .unique(None, UniqueKeepStrategy::Any)
}

pub fn semi_joins(df: DataFrame, other: LazyFrame) -> Result<DataFrame, BorrowingError> {
    // Keeps the rows only where a match is found, so that if only one of payments or disbursements is found, we can
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
