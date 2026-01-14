use polars::prelude::*;
use std::collections::{HashMap, HashSet};
static PRINCIPAL_COL: &str = "Principal Paid";
static RATE_COL: &str = "Interest Rate";
static LOAN_AMOUNT_COL: &str = "Loan amount";
static CG_COL: &str = "CG Allocation";
static CG_GST_COL: &str = "CG GST";
static LOAN_EXPS_COL: &str = "Loan expenses";
static DATE_COL: &str = "Date";
static INTEREST_PAID: &str = "Interest Paid";
static AMMENDMENT_DATE_COL: &str = "Amendment";
static CAP_DATE_COL: &str = "Capitalization Date";
pub static LOAN_ID: &str = "Loan ID";
pub fn read_csv(path: String) -> DataFrame {
    let path = std::path::PathBuf::from(path);
    let options = CsvReadOptions {
        infer_schema_length: Some(0 as usize),
        ..Default::default()
    };
    let df = CsvReader::new(std::fs::File::open(path).unwrap())
        .with_options(options)
        .finish()
        .unwrap();
    df
}

pub fn preprocess_hashmap(
    borrowings_data: &mut HashMap<String, DataFrame>,
) -> Result<(), Box<dyn std::error::Error>> {
    cast_df("payments", borrowings_data).unwrap();
    cast_df("disbursements", borrowings_data).unwrap();
    Ok(())
}

fn cast_df(
    key: &str,
    borrowings_data: &mut HashMap<String, DataFrame>,
) -> Result<(), Box<dyn std::error::Error>> {
    //-> Result<DataFrame> {
    static FLOAT_COLS: [&str; 7] = [
        LOAN_AMOUNT_COL,
        CG_COL,
        CG_GST_COL,
        LOAN_EXPS_COL,
        INTEREST_PAID,
        RATE_COL,
        PRINCIPAL_COL,
    ];
    static DATE_COLS: [&str; 3] = [DATE_COL, AMMENDMENT_DATE_COL, CAP_DATE_COL];
    let df: &mut DataFrame = &mut borrowings_data.get_mut(key).unwrap();
    let mut expressions: Vec<Expr> = Vec::new();

    for c in FLOAT_COLS {
        if df.column(c).is_ok() {
            expressions.push(
                col(c)
                    .str()
                    .replace_all(lit(","), lit(""), false)
                    .cast(DataType::Float64)
                    .alias(c),
            );
        }
    }
    for c in DATE_COLS {
        if df.column(c).is_ok() {
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
    }
    *df = std::mem::take(df)
        .lazy()
        .with_columns(expressions)
        .collect()
        .unwrap();
    Ok(())
}

pub fn process_dataframe(
    mut borrowings_data: HashMap<String, DataFrame>,
) -> Result<HashMap<String, DataFrame>, Box<dyn std::error::Error>> {
    let _x = preprocess_hashmap(&mut borrowings_data)?;
    Ok(borrowings_data)
}

pub fn split_borrowings(
    payments: DataFrame,
    disbursements: DataFrame,
) -> Result<HashMap<String, HashMap<String, DataFrame>>, Box<dyn std::error::Error>> {
    let payments_loans: HashSet<String> = get_loan_ids(&payments)?;
    let disbursements_loans: HashSet<String> = get_loan_ids(&disbursements)?;
    let valid_loans: HashSet<String> = payments_loans
        .intersection(&disbursements_loans)
        .map(|s| s.to_owned())
        .collect();
    let mut hm = HashMap::new();
    for loan in valid_loans.iter() {
        let payment = (
            "Payments".to_string(),
            payments
                .filter(
                    &payments
                        .column(LOAN_ID)?
                        .equal(&Series::new("tmp".into(), [loan as &str]).into_column())?,
                )?
                .drop(LOAN_ID)?,
        );
        let disbursement = (
            "Disbursements".to_string(),
            disbursements
                .filter(
                    &disbursements
                        .column(LOAN_ID)?
                        .equal(&Series::new("tmp".into(), [loan as &str]).into_column())?,
                )?
                .drop(LOAN_ID)?,
        );

        let loan_data: HashMap<String, DataFrame> = HashMap::from_iter([payment, disbursement]);
        hm.insert(loan.to_owned(), loan_data);
    }
    Ok(hm)
}

fn get_loan_ids(df: &DataFrame) -> Result<HashSet<String>, Box<dyn std::error::Error>> {
    Ok(df
        .column(LOAN_ID)?
        .str()
        .into_iter()
        .flatten()
        .map(|s| s.unwrap().to_string())
        .collect::<HashSet<String>>())
}
