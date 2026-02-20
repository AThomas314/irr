use crate::consts::*;
use log::{debug, info};
use polars::prelude::*;
use std::{
    collections::{BTreeMap, HashMap},
    usize,
};
pub fn read_disbursements(path: &str) -> DataFrame {
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
    ]);
    const FLOAT_COLS: [&str; 4] = [LOAN_AMOUNT_COL, CG_COL, CG_GST_COL, LOAN_EXPS_COL];
    const DATE_COLS: [&str; 3] = [DATE_COL, AMENDMENT_DATE_COL, CAP_DATE_COL];
    // const CATEGORICAL_COLS: [&str; 2] = [LOCATION, LOAN_ID];
    const SELECTOR: [&str; 7] = [
        DATE_COL,
        AMENDMENT_DATE_COL,
        CAP_DATE_COL,
        STANDALONE,
        CONSOL,
        LOCATION,
        LOAN_ID,
    ];
    let mut expressions: Vec<Expr> = Vec::with_capacity(7);

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
    // for c in CATEGORICAL_COLS {
    //     expressions.push(
    //         col(c)
    //             .cast(DataType::Categorical(
    //                 Categories::new(
    //                     PlSmallStr::from_str(c),
    //                     PlSmallStr::from_str(c),
    //                     CategoricalPhysical::U16,
    //                 ),
    //                 Arc::new(CategoricalMapping::new(u16::MAX as usize)),
    //             ))
    //             .alias(c),
    //     );
    // }
    let mut comps = Vec::with_capacity(2);
    comps.push(
        (col(LOAN_AMOUNT_COL) - col(CG_COL) - col(CG_GST_COL) - col(LOAN_EXPS_COL)).alias("Consol"),
    );
    comps.push((col(LOAN_AMOUNT_COL) - col(LOAN_EXPS_COL)).alias(STANDALONE));
    // let mut df = CsvReader::new(std::fs::File::open(path).unwrap())
    //     .with_options(options)
    //     .finish()
    //     .unwrap();
    // debug!("{:#?}", df);

    let df: LazyFrame = LazyCsvReader::new(PlPath::from_str(path))
        .with_schema(Some(Arc::from(schema)))
        .finish()
        .unwrap()
        .with_columns(expressions)
        .drop_nulls(None)
        .with_columns(comps)
        .select(SELECTOR.map(col));

    df.collect().unwrap()
}

pub fn read_payments(path: &str) -> DataFrame {
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
    // const CATEGORICAL_COLS: [&str; 1] = [LOAN_ID];
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
        // Vec::with_capacity(FLOAT_COLS.len() + DATE_COLS.len() + CATEGORICAL_COLS.len() as usize);
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
    // for c in CATEGORICAL_COLS {
    //     expressions.push(
    //         col(c)
    //             .cast(DataType::Categorical(
    //                 Categories::new(
    //                     PlSmallStr::from_str(c),
    //                     PlSmallStr::from_str(c),
    //                     CategoricalPhysical::U16,
    //                 ),
    //                 Arc::new(CategoricalMapping::new(u16::MAX as usize)),
    //             ))
    //             .alias(c),
    //     );
    // }
    let df: LazyFrame = LazyCsvReader::new(PlPath::from_str(path))
        .with_schema(Some(Arc::from(schema)))
        .finish()
        .unwrap()
        .with_columns(expressions)
        .with_column((col(INTEREST_PAID) + col(PRINCIPAL_COL)).alias(TOTAL_PAID))
        .drop_nulls(None)
        .select(SELECTOR.map(col));

    df.collect().unwrap()
}
