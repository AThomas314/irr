from numpy import ndarray , float64 ,datetime64 ,timedelta64,ones_like,zeros_like,empty_like,isin, dot, array , arange, power, ones , concat as np_concat
from traceback import print_tb
from polars import DataFrame,col,Int8,lit,Utf8,when,concat

principal_col = 'Principal Paid'
rate_col = 'Interest Rate'
loan_amount_col = 'Loan amount'
cg_col = 'CG Allocation'
cg_gst_col = 'CG GST'
loan_exps_col = 'Loan expenses'
date_col = 'Date'



def compute_irr(payments : ndarray , disbursements : ndarray , tol : float = 0.01)->float:
    def make_guess(values:ndarray[float64])->int:
        rates = array([[5],[6],[7],[8],[9],[10],[12],[15],[18],[20]])/100
        powers = arange(0,len(values),1)
        discounting_factors = 1/power(ones((len(rates),len(values)))*(rates/365.25)+1,powers) 
        idx = abs(dot(values,discounting_factors.transpose())).argmin()
        rate = rates[idx].item()
        return rate

    def newton_raphson( guess : int,
                        values : ndarray,
                        tol : float)->float:
        npv = tol*2
        powers = arange(0,len(values),1)
        onez = ones(len(values)).transpose()
        i = 1
        while abs(npv)>tol:
            discounting_factors = 1/power(onez*((guess/365.25)+1),powers)
            npv =  dot(values,discounting_factors)
            derivative = dot(values,discounting_factors/((guess/365.25)+1)*powers)*-1/365.25
            if abs(derivative) < 1e-10: #added check to avoid divide by zero.
                print("Derivative close to zero. Cannot proceed")
                print(f"Best Value was {npv} at {guess}")
                return guess
            guess-=npv/derivative
            i+=1
        # print(f'irr of {guess} found in {i} iterations')
        
        return guess
    z = array([0])
    payments = np_concat([z,payments])
    disbursements = np_concat([disbursements,z])
    values = payments - disbursements
    guess = make_guess(values)
    irr = newton_raphson(guess,values,tol)
    return irr


class Borrowing:
    __slots__ = ['payments','disbursements','location','cap_date','disbursements_consol','schedule','consol_schedule','id']

    borrowing_list = []

    def __init__(self,data,id):
        self.payments = self.process_payments(data['payments'])
        self.disbursements = self.process_disbursements(data['disbursements'] , False)
        self.disbursements_consol = self.process_disbursements(data['disbursements'] , True)
        self.location : str = list(data['disbursements'].values())[0].select('Location').item(0,0)
        self.cap_date = list(data['disbursements'].values())[0].select('Capitalization Date').item(0,0)
        self.schedule = self.compute_schedule(self.payments,self.disbursements)
        self.consol_schedule = self.compute_schedule(self.payments,self.disbursements_consol)
        self.id = id
        self.borrowing_list.append(self)

    @staticmethod
    def group_schedule(daily_schedule_df:DataFrame)-> DataFrame:
        month = col('Date').dt.strftime('%b-%Y')
        date = col('Date').dt.strftime('%d-%b-%Y')
        if 'IRR' in daily_schedule_df.columns: #Standalone
            grouped = daily_schedule_df.with_columns(
                            month.alias('Month'),
                            date.alias('From'),
                            date.alias('To'),
                            ((month!=month.shift(1).fill_null(month))|
                                (col('Payments').shift(1).fill_null(0)!=0)|
                                (col('Disbursements').fill_null(0)!=0)|
                                (col('IRR')!=col('IRR').shift(1).fill_null(col('IRR')))
                            ).cast(Int8).cum_sum().alias('group')
                        ).group_by('group',maintain_order=True).agg(
                            col('Disbursements','Interest Paid','EIR Interest','Payments','Interest Capitalized','Interest Expensed').sum(),
                            col('From','Opening Balance','Month','Location').first(),
                            col('To','Interest Rate','IRR','Closing Balance','id').last(),
                            col('group').count().alias('Days')
                        )
            l=grouped.select('IRR','Interest Rate').unique()
            mapper = {i[0]:i[1] for i in l.iter_rows() if i[1]!=0}
            def get_map(key):
                return mapper.get(key, 0.0)
            grouped=grouped.with_columns(col('IRR').map_elements(get_map,float).alias('Interest Rate'))
            grouped=grouped.with_columns((col('IRR','Interest Rate')*100).cast(Utf8)+ lit("%"))
        else: #Consol
            grouped = daily_schedule_df.with_columns(
                            month.alias('Month'),
                            date.alias('From'),
                            date.alias('To'),
                            ((month!=month.shift(1).fill_null(month)) | (col('Payments').shift(1).fill_null(0)!=0)| (col('Disbursements').fill_null(0)!=0) ).cast(Int8).cum_sum().alias('group')
                    ).group_by('group',maintain_order=True).agg(
                        col('Disbursements','Interest Paid','EIR Interest','Payments','Interest Capitalized','Interest Expensed').sum(),
                        col('From','Opening Balance','Month','Location').first(),
                        col('To','Closing Balance').last(),
                        col('group').count().alias('Days')
                    )
        amort = col('EIR Interest')-col('Interest Paid')
        grouped = grouped.with_columns(
                amort.alias('Amortization'),
                )
        amortmap=grouped.group_by('Month').agg(col('Amortization','Days').sum()).with_columns((col('Amortization')/col('Days')).alias('DailyAmortization')).select('Month','DailyAmortization')
        grouped=grouped.join(amortmap,on='Month').with_columns((col('DailyAmortization')*col('Days')).alias('Amortization')).drop('DailyAmortization').with_columns(
                col('Amortization').cum_sum(reverse=True).alias('Opening Unamortized Transaction Costs'),
                (col('Amortization').cum_sum(reverse=True)-col('Amortization')).alias('Closing Unamortized Transaction Costs')
        )
        col_order=[i for i in ['id','Location','Month','From','To','Days','Interest Rate','IRR','Opening Balance','Disbursements','EIR Interest','Payments','Closing Balance','Principal Paid','Interest Paid','Opening Unamortized Transaction Costs','Amortization','Closing Unamortized Transaction Costs','Interest Expensed','Interest Capitalized'] if i in grouped.columns]
        grouped = grouped.select(col_order)
        return grouped

    def to_dataframe(self,consol:bool = False)->DataFrame:
        if not consol:
            df = DataFrame(self.schedule,schema=['Date','Interest Rate','Interest Paid','Opening Balance','Disbursements','EIR Interest','Payments','Closing Balance','IRR'])

        else:
            df = DataFrame(self.consol_schedule,schema =  ['Date','Interest Rate','Interest Paid','Opening Balance','Disbursements','EIR Interest','Payments','Closing Balance','IRR'])
        df = df.with_columns(
            lit(self.location).alias('Location'),
            lit(self.id).alias('id')
        )

        try:
            if self.cap_date not in ['',None]:
                df = df.with_columns(
                    when(col('Date')<=self.cap_date).then(col('EIR Interest')).otherwise(0).alias('Interest Capitalized'),
                    when(col('Date')>self.cap_date).then(col('EIR Interest')).otherwise(0).alias('Interest Expensed')
                    )
            elif self.cap_date in['',None]:
                df = df.with_columns(
                    col('EIR Interest').alias('Interest Expensed'),
                    lit(0).alias('Interest Capitalized'),
                )
        except TypeError:
            pass
        except Exception as e:
            print(e)
            print(e.__doc__)
            print_tb(e.__traceback__)
        return df

    @staticmethod
    def process_payments(payments_data : dict[DataFrame]) -> dict[tuple[ndarray[float64|datetime64]]]:
        data = {}
        for key in payments_data.keys():
            df = payments_data[key]
            principal : ndarray[float64]  = df.select(principal_col).to_numpy().squeeze()
            int_rate : ndarray[float64]  = df.select(rate_col).to_numpy().squeeze()
            int_paid : ndarray[float64]  = df.select('Interest Paid').to_numpy().squeeze()
            payment_dates : ndarray[datetime64]  = df.select(date_col).to_numpy().squeeze()
            amendment_dates : ndarray[datetime64]  = df.select('Amendment').to_numpy().squeeze()
            data[key] = int_rate , principal , int_paid , payment_dates , amendment_dates
        return data
    
    @staticmethod
    def process_disbursements(  disbursements_data:dict[DataFrame],
                                consol : bool)->dict[tuple[ndarray[float64|datetime64]]]:
        data = {}
        for key in disbursements_data.keys():
            df = disbursements_data[key]
            loan_amount : ndarray[float64] = df.select(loan_amount_col).to_numpy().squeeze()
            cg_gst : ndarray[float64] = df.select(cg_gst_col).to_numpy().squeeze()
            loan_expenses : ndarray[float64]  = df.select(loan_exps_col).to_numpy().squeeze()
            disbursement_dates : ndarray[datetime64]  = df.select(date_col).to_numpy().squeeze()
            amendment_dates : ndarray[datetime64]  = df.select('Amendment').to_numpy().squeeze()
            if consol:
                expenses = loan_expenses + cg_gst
                disbursements : ndarray[float64]  = loan_amount - expenses
            else:
                cg_amount : ndarray[float64] = df.select(cg_col).to_numpy().squeeze()
                expenses = loan_expenses + cg_gst + cg_amount
                disbursements : ndarray[float64]  = loan_amount - expenses
            data[key] =  disbursements , disbursement_dates , amendment_dates
        return data

    @staticmethod    
    def compute_schedule(   payments : dict[tuple[ndarray[float64|datetime64]]],
                            disbursements : dict[tuple[ndarray[float64|datetime64]]])->tuple[ndarray[float64,datetime64]]:
        def get_date_series(payment_dates,disbursement_dates):
            #No JIT
            start_date : datetime64 = min(payment_dates.min(),disbursement_dates.min())
            end_date : datetime64 = max(payment_dates.max(),disbursement_dates.max())
            date_series = arange(start_date,end_date + timedelta64(1,'D'),timedelta64(1,'D'))
            return date_series

        def padded_cashflows(   date_series : ndarray[datetime64],
                                payment_dates : ndarray[datetime64],
                                disbursement_dates : ndarray[datetime64],
                                payments : ndarray[float64],
                                disbursements : ndarray[float64],
                                int_rate : ndarray[float64],
                                int_paid : ndarray[float64])->tuple[ndarray]: 
            '''Create an array of zeroes and fill values corresponding with dates with that dates payment/disbursement'''
            payments_padded = zeros_like(date_series,dtype = float64)
            disbursements_padded = zeros_like(date_series,dtype = float64)
            int_rate_padded = zeros_like(date_series,dtype = float64)
            int_paid_padded = zeros_like(date_series,dtype = float64)
            payment_indices = arange(len(date_series))[isin(date_series,payment_dates)]
            payments_padded[payment_indices] = payments
            int_rate_padded[payment_indices] = int_rate
            int_paid_padded[payment_indices] = int_paid
            disbursement_indices = arange(len(date_series))[isin(date_series,disbursement_dates)]
            disbursements_padded[disbursement_indices] = disbursements
            return payments_padded , disbursements_padded , int_rate_padded , int_paid_padded
        
        def compute_daily_schedule( date_series : ndarray,
                                    payments_padded : ndarray,
                                    disbursements_padded : ndarray,
                                    int_rate_padded : ndarray,
                                    int_paid_padded : ndarray,
                                    irr : float,
                                    )->tuple:
            opbal : ndarray = empty_like(payments_padded)
            clbal : ndarray = empty_like(disbursements_padded)
            interest : ndarray = empty_like(payments_padded)
            opbal[0] = 0.0
            for i in range(len(opbal)):
                interest[i] = ((opbal[i] + disbursements_padded[i]) * irr).item()
                clbal[i] = opbal[i] + interest[i] + disbursements_padded[i] - payments_padded[i]
                if i + 1 < len(opbal):
                    opbal[i+1] = clbal[i]
            clbal[-1] = 0
            __irr__ : ndarray = ones_like(opbal)*irr*365.25
            return date_series , int_rate_padded , int_paid_padded.round() , opbal.round() , disbursements_padded.round() , interest.round() , payments_padded.round() , clbal.round() , __irr__

        for i in range(len(payments)):
            j = list(payments.keys())[i]
            int_rate = payments[j][0]
            principal_paid =  payments[j][1]
            int_paid = payments[j][2]
            payment_dates = payments[j][3]

            if i == 0 :
                disbursements_amounts = array([disbursements[j][0]])
                disbursements_dates = array([disbursements[j][1]])
                date_series = get_date_series(payment_dates , disbursements_dates)
                net_payments = int_paid + principal_paid
                payments_padded , disbursements_padded , int_rate_padded,int_paid_padded =\
                    padded_cashflows(date_series , payment_dates , disbursements_dates,net_payments,disbursements_amounts,int_rate,int_paid)
                
                irr = compute_irr(payments_padded,disbursements_padded)/365.25
                date_series , int_rate_padded , int_paid_padded , opbal , disbursements_padded , interest , payments_padded,clbal,__irr__ = compute_daily_schedule(date_series,payments_padded,disbursements_padded,int_rate_padded,int_paid_padded,irr)
            if i > 0:
                int_rate = payments[j][0]
                principal_paid =  payments[j][1]
                int_paid = payments[j][2]
                payment_dates = payments[j][3]
                amendment_date = payments[j][4][0]
                if (amendment_date > date_series.max()) or (amendment_date<date_series.min()):
                    raise ValueError ('Current start date outside the daterange, please rectify the input files')
                idx : int = date_series.searchsorted(amendment_date)+1    
                date_series_orig = date_series[:idx]
                int_rate_padded_orig = int_rate_padded[:idx]
                int_paid_padded_orig = int_paid_padded[:idx]
                opbal_orig = opbal[:idx]
                disbursements_padded_orig = disbursements_padded[:idx]
                interest_orig = interest[:idx]
                payments_padded_orig = payments_padded[:idx]
                clbal_orig = clbal[:idx]
                __irr__orig = __irr__[:idx]
                try:                    
                    disbursements_amounts = disbursements[j][0]
                    if isinstance(disbursements_amounts,float64):
                        disbursements_amounts = array([disbursements_amounts])
                    elif isinstance(disbursements_amounts,ndarray):
                        if len(disbursements_amounts)==1:
                            disbursements_amounts = array([disbursements_amounts])
                    disbursements_dates = disbursements[j][1]
                    if amendment_date in disbursements_dates:
                        disb_on_amend_date = disbursements_amounts[0]
                        disbursements_amounts[0]+=opbal_orig[-1]
                    else:
                        disb_on_amend_date = 0
                        disbursements_dates = np_concat(array([amendment_date]),disbursements_dates)
                        disbursements_amounts = np_concat(array([opbal_orig[-1]]),disbursements_dates)
                except KeyError:
                    disb_on_amend_date = 0
                    disbursements_amounts = array([opbal_orig[-1]])
                    disbursements_dates = array([date_series_orig[-1]])
                date_series = get_date_series(payment_dates , disbursements_dates)
                net_payments = int_paid + principal_paid
                payments_padded , disbursements_padded , int_rate_padded,int_paid_padded =\
                    padded_cashflows(date_series , payment_dates , disbursements_dates,net_payments,disbursements_amounts,int_rate,int_paid)
                irr = compute_irr(payments_padded,disbursements_padded)/365.25
                date_series , int_rate_padded , int_paid_padded , opbal , disbursements_padded , interest , payments_padded,clbal,__irr__ = compute_daily_schedule(date_series,payments_padded,disbursements_padded,int_rate_padded,int_paid_padded,irr)
                date_series = np_concat((date_series_orig[:-1] , date_series))
                int_rate_padded = np_concat((int_rate_padded_orig[:-1] , int_rate_padded))
                int_paid_padded = np_concat((int_paid_padded_orig[:-1] , int_paid_padded))
                opbal = np_concat((opbal_orig,opbal[1:]))
                disbursements_padded_orig[-1] = disb_on_amend_date
                disbursements_padded = np_concat((disbursements_padded_orig,disbursements_padded[1:]))
                interest = np_concat((interest_orig[:-1],interest))
                payments_padded = np_concat((payments_padded_orig[:-1],payments_padded))
                clbal =  np_concat((clbal_orig[:-1],clbal))
                __irr__ = np_concat((__irr__orig[:-1],__irr__))
            daily_schedule = date_series , int_rate_padded , int_paid_padded , opbal , disbursements_padded , interest , payments_padded,clbal,__irr__
        return daily_schedule

    @classmethod
    def consol(cls)->DataFrame:
        dfs = [borrowing.to_dataframe(consol=True
                    ).with_columns(
                    lit(borrowing.location).alias('Location')
                ) for borrowing in Borrowing.borrowing_list
                ]
        dfs = concat(dfs)
        dfs = dfs.group_by(['Location','Date']
                            ).agg(
                                col('Interest Paid','Opening Balance','Disbursements','EIR Interest','Payments','Closing Balance','Interest Capitalized','Interest Expensed').sum()
                            )
        return dfs
    
    def journal_entry(self,consol:bool,month:str):
#        month to be in format %b-%Y
        df = self.to_dataframe(consol)
        grouped_schedule = self.group_schedule(df)
        grouped_schedule = grouped_schedule.filter(col('Month')==month
                                                ).group_by('Month'
                                                ).agg(
                                                col('EIR Interest', 'Payments','Interest Capitalized','Interest Expensed').sum()
                )
        eir = grouped_schedule.select('EIR Interest').item()
        payment = grouped_schedule.select('Payments').item()
        expense = grouped_schedule.select('Interest Expensed').item()
        capitalise = grouped_schedule.select('Interest Capitalized').item()
        
        entry = DataFrame([['EIR',eir,'',f'EIR of {eir} accrued against loan {self.id}'],
                           ['To Borrowings','',eir,f'EIR of {eir} accrued against loan {self.id}'],
                           ['Borrowings',payment,'',f'Payment of {payment} made against loan {self.id}'],
                           ['To Bank','',payment,f'Payment of {payment} made against loan {self.id}'],
                           ['Profit and Loss',expense,'','EIR interest booked to profit and loss account'],
                           ['Eligible Asset',capitalise,'','EIR interest capitalised against eligible asset'],
                           ['EIR','',eir,f'EIR interest booked to profit and loss account to the extent of {expense} and capitalized to the extent of {capitalise}']],
                           schema=['Particulars','Debit','Credit','Narration'],orient='row')
        return entry