import matplotlib.pyplot as plt
import pandas as pd


def main():
    try:
        df_rollup = pd.read_csv('../gas_reports/rollup_gas_report.csv', header=0)
        df_validium = pd.read_csv('../gas_reports/validium_gas_report.csv', header=0)
    except:
        print("Error: Report not found.")
        return

    rollup_mean_transaction_gas = df_rollup.groupby('operation')['transaction_gas_used'].mean().reset_index()
    validium_mean_transaction_gas = df_validium.groupby('operation')['transaction_gas_used'].mean().reset_index()

    results_df = pd.merge(rollup_mean_transaction_gas, validium_mean_transaction_gas, on='operation', suffixes=('_rollup', '_validium'))

    results_df.plot(x='operation', 
        kind='bar', 
        stacked=False) 
    plt.savefig('../gas_reports/report_graph.png')

if __name__ == "__main__":
    main()
