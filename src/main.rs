mod comps;
mod consts;
mod errors;
use errors::BorrowingError;
mod funcs;
// use comps::compute_irr;
use funcs::*;
use log::debug;

use tokio;
mod structs;
use structs::*;
#[tokio::main]
async fn main() -> Result<(), BorrowingError> {
    env_logger::init(); //initialize the logger to be used in debug and prod

    let (payments, disbursements) = tokio::join!(
        tokio::task::spawn_blocking(move || read_payments(
            r"C:\Users\ashis\OneDrive\Desktop\rust\irr\payments 1116 final 1 1.csv"
        )),
        tokio::task::spawn_blocking(move || read_disbursements(
            r"C:\Users\ashis\OneDrive\Desktop\rust\irr\disbursements 1116 3.csv"
        ))
    ); //Read the files in parallel
    let borrowings = Borrowings::new(payments??, disbursements??)?; //Create the borrowings struct
    // debug!("{:#?}", borrowings);
    // info!("Borrowings {:#?}", borrowings);
    // Borrowing::process(&mut borrowings);
    borrowings.process()?;
    Ok(())
}
