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

mod comps;
mod consts;
mod errors;
use clap::Parser;
use errors::BorrowingError;
mod funcs;
// use comps::compute_irr;
use funcs::*;
use tokio::{self, time::Instant};
mod structs;
use structs::*;

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> Result<(), BorrowingError> {
    env_logger::init();
    let paths = FilePaths::parse();
    let payments_path = paths.payments_path;
    let disbursements_path = paths.disbursements_path;
    let (payments, disbursements) = tokio::join!(
        tokio::task::spawn_blocking(move || read_payments(&payments_path)),
        tokio::task::spawn_blocking(move || read_disbursements(&disbursements_path))
    ); //Read the files in parallel
    let borrowings = Borrowings::new(payments??, disbursements??)?; //Create the borrowings struct
    borrowings.process()?; //process the borrowings
    let start = Instant::now(); //for benchmarking
    for _ in 0..100 {
        borrowings.process()?
    }
    println!("Average {:#?}", start.elapsed() / 100);
    Ok(())
}
