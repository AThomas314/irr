use chrono::NaiveDate;
use log::{debug, info};
use polars::prelude::*;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    usize,
};
pub static DISBURSEMENTS: &str = "Disbursements";
pub static PAYMENTS: &str = "Payments";
pub static LOAN_ID: &str = "Loan ID";
pub static LOCATION: &str = "Location";
pub static DATE_COL: &str = "Date";
pub static AMENDMENT_DATE_COL: &str = "Amendment";
static PRINCIPAL_COL: &str = "Principal Paid";
static RATE_COL: &str = "Interest Rate";
static LOAN_AMOUNT_COL: &str = "Loan amount";
static CG_COL: &str = "CG Allocation";
static CG_GST_COL: &str = "CG GST";
static LOAN_EXPS_COL: &str = "Loan expenses";
static INTEREST_PAID: &str = "Interest Paid";
static CAP_DATE_COL: &str = "Capitalization Date";

pub async fn read_disbursements(path: String) -> DataFrame {
    let path = std::path::PathBuf::from(path);
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
    let options = CsvReadOptions {
        infer_schema_length: Some(0 as usize),
        schema: Some(Arc::new(schema)),
        parse_options: Arc::new(CsvParseOptions {
            truncate_ragged_lines: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    static FLOAT_COLS: [&str; 4] = [LOAN_AMOUNT_COL, CG_COL, CG_GST_COL, LOAN_EXPS_COL];
    static DATE_COLS: [&str; 3] = [DATE_COL, AMENDMENT_DATE_COL, CAP_DATE_COL];
    static CATEGORICAL_COLS: [&str; 2] = [LOCATION, LOAN_ID];
    let mut expressions: Vec<Expr> =
        Vec::with_capacity(FLOAT_COLS.len() + DATE_COLS.len() + CATEGORICAL_COLS.len() as usize);
    let mut df = CsvReader::new(std::fs::File::open(path).unwrap())
        .with_options(options)
        .finish()
        .unwrap();
    debug!("{:#?}", df);
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
    for c in CATEGORICAL_COLS {
        expressions.push(
            col(c)
                .cast(DataType::Categorical(
                    Categories::new(
                        PlSmallStr::from_str(c),
                        PlSmallStr::from_str(c),
                        CategoricalPhysical::U16,
                    ),
                    Arc::new(CategoricalMapping::new(u16::MAX as usize)),
                ))
                .alias(c),
        );
    }
    df = df.lazy().with_columns(expressions).collect().unwrap();
    df
}

pub async fn read_payments(path: String) -> DataFrame {
    let path = std::path::PathBuf::from(path);
    let schema = Schema::from_iter(vec![
        Field::new(PlSmallStr::from_str(DATE_COL), DataType::String),
        Field::new(PlSmallStr::from_str(RATE_COL), DataType::String),
        Field::new(PlSmallStr::from_str(INTEREST_PAID), DataType::String),
        Field::new(PlSmallStr::from_str(PRINCIPAL_COL), DataType::String),
        Field::new(PlSmallStr::from_str(LOAN_ID), DataType::String),
        Field::new(PlSmallStr::from_str(AMENDMENT_DATE_COL), DataType::String),
    ]);
    let options = CsvReadOptions {
        infer_schema_length: Some(0 as usize),
        schema: Some(Arc::new(schema)),
        parse_options: Arc::new(CsvParseOptions {
            truncate_ragged_lines: true,
            ..Default::default()
        }),

        ..Default::default()
    };
    static FLOAT_COLS: [&str; 3] = [INTEREST_PAID, RATE_COL, PRINCIPAL_COL];
    static DATE_COLS: [&str; 2] = [DATE_COL, AMENDMENT_DATE_COL];
    static CATEGORICAL_COLS: [&str; 1] = [LOAN_ID];
    let mut expressions: Vec<Expr> =
        Vec::with_capacity(FLOAT_COLS.len() + DATE_COLS.len() + CATEGORICAL_COLS.len() as usize);

    let mut df = CsvReader::new(std::fs::File::open(path).unwrap())
        .with_options(options)
        .finish()
        .unwrap();
    debug!("{:#?}", df);
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
    for c in CATEGORICAL_COLS {
        expressions.push(
            col(c)
                .cast(DataType::Categorical(
                    Categories::new(
                        PlSmallStr::from_str(c),
                        PlSmallStr::from_str(c),
                        CategoricalPhysical::U16,
                    ),
                    Arc::new(CategoricalMapping::new(u16::MAX as usize)),
                ))
                .alias(c),
        );
    }

    df = df.lazy().with_columns(expressions).collect().unwrap();
    df
}

pub fn split_borrowings(
    payments: DataFrame,
    disbursements: DataFrame,
) -> Result<
    HashMap<String, HashMap<String, BTreeMap<NaiveDate, DataFrame>>>,
    Box<dyn std::error::Error>,
> {
    // {"Loan ID" :{"Payments" :payments_df,"Disbursements":disbursements_df}}
    let payments_loans: HashSet<String> = get_loan_ids(&payments).unwrap();
    debug!("payments_loans {:#?}", payments_loans);
    let disbursements_loans: HashSet<String> = get_loan_ids(&disbursements)?;
    debug!("disbursements_loans {:#?}", disbursements_loans);
    let valid_loans: HashSet<String> = payments_loans
        .intersection(&disbursements_loans)
        .map(|s| s.to_owned())
        .collect();
    let mut hm: HashMap<String, HashMap<String, BTreeMap<NaiveDate, DataFrame>>> = HashMap::new();
    for loan in valid_loans.iter() {
        let payments: BTreeMap<NaiveDate, DataFrame> = create_amendment_split(loan, &payments)?;
        debug!("payments {:#?}", payments);
        let disbursements: BTreeMap<NaiveDate, DataFrame> =
            create_amendment_split(loan, &disbursements)?;
        debug!("disbursements {:#?}", disbursements);
        let mut loan_data = HashMap::new();
        loan_data.insert(PAYMENTS.to_string(), payments);
        loan_data.insert(DISBURSEMENTS.to_string(), disbursements);
        hm.insert(loan.to_owned(), loan_data);
    }
    Ok(hm)
}

fn get_loan_ids(df: &DataFrame) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    debug!(
        "{:#?}",
        df.column(LOAN_ID)?
            .as_materialized_series()
            .unique()
            .unwrap()
            .cast(&DataType::String)
            .unwrap()
            .str()?
            .into_iter()
            .flatten()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>()
    );
    Ok(        df.column(LOAN_ID)?
            .as_materialized_series()
            .unique()
            .unwrap()
            .cast(&DataType::String)
            .unwrap()
            .str()?
            .into_iter()
            .flatten()
            .map(|s| s.to_string())
            .collect::<HashSet<String>>()
)
}

fn create_amendment_split(
    loan: &str,
    df: &DataFrame,
) -> Result<BTreeMap<NaiveDate, DataFrame>, Box<dyn std::error::Error>> {
    // Amendment Date ->DataFrame

    let df = df
        .filter(
            &df.column(LOAN_ID)?
                .equal(&Series::new("tmp".into(), [loan as &str]).into_column())?,
        )
        .unwrap()
        .drop(LOAN_ID)?;
    debug!("Filtered {:#?}", df);
    let partitions = df.partition_by([AMENDMENT_DATE_COL], true)?;
    let mut bm = BTreeMap::new();
    for part in partitions {
        let days: i32 = part
            .column(AMENDMENT_DATE_COL)?
            .date()?
            .get_any_value(0)?
            .try_extract::<i32>()?;
        let date =
            NaiveDate::from_ymd_opt(1970, 1, 1).unwrap() + chrono::Duration::days(days as i64);
        bm.insert(date, part.drop(AMENDMENT_DATE_COL)?);
    }

    Ok(bm)
}
