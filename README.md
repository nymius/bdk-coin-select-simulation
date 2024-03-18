# bdk-coin-selection-simulation
---
A simulator to test [bitcoindevkit/coin-select](https://github.com/bitcoindevkit/coin-select) against coin selection in Bitcoin Core and gain introspection about this Rust coin selection implementation.

## Overview
---

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
