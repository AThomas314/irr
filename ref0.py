    def process_loans(self):
        loans = {}
        for loan_id in self.payments_df.select("Loan ID").unique().to_series():
            loans[loan_id] = {}
            loans[loan_id]["payments"] = {}
            loan_df = self.payments_df.filter(col("Loan ID") == loan_id)
            Amendment_dates = (
                loan_df.select(col("Amendment")).unique().sort("Amendment").to_series()
            )
            for i in range(len(Amendment_dates)):
                Amendment_df = loan_df.filter(col("Amendment") == Amendment_dates[i])
                loans[loan_id]["payments"][i] = Amendment_df
        for loan_id in self.disbursements_df.select("Loan ID").unique().to_series():
            try:
                loans[loan_id]["disbursements"] = {}
                loan_df = self.disbursements_df.filter(col("Loan ID") == loan_id)
                Amendment_dates = (
                    loan_df.select(col("Amendment"))
                    .unique()
                    .sort("Amendment")
                    .to_series()
                )
                for i in range(len(Amendment_dates)):
                    Amendment_df = loan_df.filter(
                        col("Amendment") == Amendment_dates[i]
                    )
                    loans[loan_id]["disbursements"][i] = Amendment_df
            except KeyError:
                pass
        data = [(loans[loan], loan) for loan in loans if len(loans[loan].keys()) == 2]

