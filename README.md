# bdk-coin-selection-simulation
A simulator to test [bitcoindevkit/coin-select](https://github.com/bitcoindevkit/coin-select) against coin selection in Bitcoin Core and gain introspection about this Rust coin selection implementation.

## Overview

Currently there is no production ready bitcoin coin selection available in Rust. There are different ongoing efforts and others have been done in the past but none of them achieved the desired state:
- [bitcoindevkit/coin-select](https://github.com/bitcoindevkit/coin-select)
- [p2pderivatives/rust-bitcoin-coin-selection](https://github.com/p2pderivatives/rust-bitcoin-coin-selection)
- [rust-coinselect](https://github.com/Bitshala-Incubator/rust-coinselect)

On the other side, Bitcoin Core coin selection module is the one with more innovation, test and analysis than any of the other ones. It has the greatest number of selection algorithms available:
- Knapsack
- [Single Random Draw](https://github.com/bitcoin/bitcoin/pull/17526)
- [Branch and Bound](https://github.com/bitcoin/bitcoin/pull/10637)
- [Coin Grinder](https://github.com/bitcoin/bitcoin/pull/27877).
- [Gutter Guard](https://github.com/bitcoin/bitcoin/pull/28977) (coming).

There is at least one simulator implemented: [coin-selection-simulation](https://github.com/achow101/coin-selection-simulation).
And the results obtained from those simulations are being actively researched: [Delving Bitcoin: GutterGuard and CoinGrinder results](https://delvingbitcoin.org/t/gutterguard-and-coingrinder-simulation-results/279).

It seems the current "~~gold~~ bitcoin standard" of coin selection is Bitcoin Core, and as it, the goal of any new coin selection implementation should be to be at least as good.

One step toward this goal is the disposal of similar results to the one obtained by [coin-selection-simulation](https://github.com/achow101/coin-selection-simulation) .

### First iteration
As the first iteration to implement a simulator with Rust coin selection compatibility and as there has already been [interest announced on it]( https://github.com/bitcoindevkit/coin-select/pull/21#issuecomment-1915811752), the target implementation is [bitcoindevkit/coin-select](https://github.com/bitcoindevkit/coin-select).

As the short term objectives is to be comparable with Bitcoin Core coin selection the simulator should be designed to produce a comparable output. So, the main code source of inspiration for it is [coin-selection-simulation](https://github.com/achow101/coin-selection-simulation).

### Further iterations
A more ambitious objective is to have a simulation framework for coin selection written in Rust, which is capable of testing different coin selection algorithms implemented in different languages, producing a common output format useful for anyone trying to gain insight from it.

## Implementation

As said above, as a first iteration of the simulator, the goal is to obtain a comparable result to the obtained with the [coin-selection-simulation](https://github.com/achow101/coin-selection-simulation). The implementation is mainly is mainly a code transliteration with some adjustments trying to avoid the output modifications as much as possible.

### Input files
```text
data
└── scenarios
    ├── bustabit-2019-2020-tiny.csv
    └── random_blocks.csv
```

A selection of the shortest scenarios available in [coin-selection-simulation](https://github.com/achow101/coin-selection-simulation).
Each file is organized in two columns.
The first column is composed of signed floating numbers up to 8 decimal places where the sign tell us if the amount, expressed in bitcoin units, is a deposit or a withdrawal.
The second column is also a quantity expressed in bitcoin units, an unsigned floating point number with up to 8 decimal places, but giving the fee rate at which the deposit or withdrawal was done. Fees on deposits doesn't have any effect while in withdrawals must be accounted at the moment of considering inputs and change outputs.
For now, there is no control on the amount of outputs included on each transaction. The simulator is going to try to payment as they arrive, and if cannot be honored, will be queued up to pay together with the next payment arrival.
### Output files
```text
simulation_results/
├── full_results.csv
├── inputs.csv
├── results.csv
└── utxos.csv
```

Where:
- `full_results.csv`: a record of each coin selection attempt, including the failed ones.
- `inputs.csv`: a list of the input amounts used on each selection.
- `results.csv`: a sample summary done after each 500 successfully selections.
- `utxos.csv`: the state of the UTxO set before each coin selection try.

#### `full_results.csv` fields
- `id`: a unique identifier for the selection attempt. Here we use the selection attempt instead of the effective successful selection, as it is done in [coin-selection-simulation](https://github.com/achow101/coin-selection-simulation) because [bitcoindevkit/coin-select](https://github.com/bitcoindevkit/coin-select) just has two different coin selection algorithms implemented:
	- Branch and bound
	- First In first out based on the order of the candidates.
  less than the available in Bitcoin Core, and not as performant, deriving in a higher number of failed selection attempts.
  The attempts count is used instead of successful withdrawals to make selection failures visible.
- `amount`: the total amount required for the withdrawal.
- `fee`: the total fee associated with the produced transaction after coin selection.
- `target_feerate`: the fee rate to aim in this selection. This value is fixed previous to the coin selection to avoid solving a multi objective non linear problem.
- `real_feerate`: the actual fee rate the transaction obtained by the coin selection process will have.
- `algorithm`: the name of the algorithm producing the successful selection or `failed` if wasn't possible to produce one.
- `input_count`: the number of inputs selected to fulfill the withdrawal.
- `negative_effective_valued_utxos`: the number of UTxOs selected as input that accounted for negative amounts in the selection at the feerate at which the selection was produced.
- `output_count`: the number of outputs included in the to-be-created transaction. In the current implementation, this only changes by accumulation of payments or addition of change outputs.
- `change_amount`: the amount of excess bitcoin returned in a change output.
- `utxo_count_before_payment`: the count of UTxOs in the UTxO set before producing the selection.
- `utxo_count_after_payment`: the resultant count of UTxOs after coin selection.
- `waste_score`: a metric designed to compare different coin selections accounting for the timing cost of creating a transaction at a determined fee rate in relation to a long term established fee rate and the creation costs associated to the inclusion or not of a change output, that might need to be spend in the future.

#### `results.csv` fields:
- `scenario_file`: the name of the simulated scenario.
- `current_balance`: the available balance at the moment of the sampling.
- `current_utxo_set_count`: the number of available UTxOs at the moment of the sampling.
- `deposit_count`: the number of deposits reached so far.
- `input_spent_count`: the total amount of UTxOs selected as transaction inputs.
- `withdraw_count`: the number of successful selections.
- `negative_effective_valued_utxos_spent_count`: the number of UTxOs that were included to the transaction without util payload, i.e., which fees to include them consumed all the valued they carry and probably value from other UTxOs.
- `created_change_outputs_count`: the total of change outputs created.
- `changeless_transaction_count`: the transactions which didn't include any change outputs.
- `min_change_value`: the minimum amount of bitcoin included in a change output.
- `max_change_value`: the maximum amount of bitcoin included in a change output.
- `mean_change_value`: the change amount mean.
- `std_dev_of_change_value`: the standard deviation of the values of the change outputs produced.
- `total_fees`: the accumulated amount of fees paid.
- `mean_fees_per_withdraw`: mean of the fees paid per withdrawal.
- `cost_to_empty_at_long_term_fee_rate`: the amount of fees which would take to include all the remaining UTxOs in the UTxO set as inputs for a transaction at the long term fee rate.
- `total_cost`: the fees paid so far plus the cost to empty the UTxO set at the current fee rate.
- `min_input_size`: the minimum amount of selected inputs.
- `max_input_size`: the maximum amount of selected inputs.
- `mean_input_size`: the mean amount of selected inputs.
- `std_dev_of_input_size`: the standard deviation of the amount of selected inputs.
- `usage`: a digest of the amount of times an algorithm was used in a successful selection or failed.


## Simulated algorithm
Currently there is no way to change the algorithm being run without touching the code.

The simulated algorithm is Branch and Bound optimizing to get a selection with the lowest fees incurred now and in the future when spending the possibly created change output.

The change policy decides based on waste and only includes a change output when it decreases the excess given away as part of the fees.

The hard limit of Branch and Bound iterations is 100000, after which if there is no solution the algorithm fails and a FIFO solution is searched based on the sorting order of the candidates.

Following [coin-selection-simulation](https://github.com/achow101/coin-selection-simulation) , the only types of UTxOs used are P2WPKH.

## Usage
To execute a simulation run:
```bash
./simulate.sh
```
From the root of the git repository.

It will build a release optimized version of the code and execute a simulation using the `bustabit-2019-2020-tiny.csv` scenario file. The output is going to be saved in the `simulation_results` directory.

The execution should start afterward. If a `File exists (os error 17)` error appears instead, remove or rename the `./simulation_results` directory and re-execute the command.
