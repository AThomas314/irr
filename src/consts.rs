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

/// All the constants that will be required are defined seperately here
//Input DataFrames
pub const LOAN_ID: &str = "Loan ID";
pub const LOCATION: &str = "Location";
pub const DATE_COL: &str = "Date";
pub const AMENDMENT_DATE_COL: &str = "Amendment";
pub const PRINCIPAL_COL: &str = "Principal Paid";
pub const RATE_COL: &str = "Interest Rate";
pub const LOAN_AMOUNT_COL: &str = "Loan amount";
pub const CG_COL: &str = "CG Allocation";
pub const CG_GST_COL: &str = "CG GST";
pub const LOAN_EXPS_COL: &str = "Loan expenses";
pub const INTEREST_PAID: &str = "Interest Paid";
pub const CAP_DATE_COL: &str = "Capitalization Date";
pub const TOTAL_PAID: &str = "Total Paid";
pub const CONSOL: &str = "Consol";
pub const STANDALONE: &str = "Standalone";
//Output DataFrames
pub const CL_BAL: &str = "Closing Balance";
pub const PAYMENTS: &str = "Payments";
pub const DISBURSEMENTS: &str = "Disbursements";
pub const IRR: &str = "IRR";
pub const INT_RATE: &str = "Interest Rate";
pub const PRINC_PAID: &str = "Principal Paid";
pub const INT_PAID: &str = "Interest Paid";
pub const EIR_INT: &str = "EIR Interest";
pub const OP_BAL: &str = "Opening Balance";
pub const ID: &str = "id";
pub const INT_EXP: &str = "Interest Expensed";
pub const INT_CAP: &str = "Interest Capitalized";
pub const MONTH: &str = "Month";
pub const GROUP: &str = "Group";
pub const FROM: &str = "From";
pub const TO: &str = "To";
pub const DAYS: &str = "Days";
pub const AMORTIZATION: &str = "Amortization";
pub const INV_365_25: f64 = 1.0 / 365.25;
