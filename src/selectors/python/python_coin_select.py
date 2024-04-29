from bitcoin_coin_selection.selection_algorithms.select_coins import (
  select_coins
)
from bitcoin_coin_selection.selection_types.coin_selection import CoinSelection
from bitcoin_coin_selection.selection_types.coin_selection_params import CoinSelectionParams
from bitcoin_coin_selection.selection_types.output_group import OutputGroup
from bitcoin_coin_selection.selection_types.input_coin import InputCoin
from bitcoin_coin_selection.selection_types.change_constants import MAX_MONEY
from bitcoin_coin_selection.selection_types.coin_selection import (
    CoinSelection
)

from bitcoin_coin_selection.selection_algorithms.branch_and_bound import select_coins_branch_and_bound
from bitcoin_coin_selection.selection_algorithms.knapsack_solver import select_coins_knapsack_solver
from bitcoin_coin_selection.selection_algorithms.single_random_draw import select_coins_single_random_draw
from bitcoin_coin_selection.selection_types.coin_selection_params import (
    CoinSelectionParams
)



from bitcoinlib.values import Value

SEGWIT_V1_TXIN_WEIGHT = 68
SEGWIT_V1_TXOUT_WEIGHT = 31

def varint_size(v: int) -> int:
    if v <= 0xfc:
        return 1
    if v <= 0xffff:
        return 3
    if v <= 0xffff_ffff:
        return 5
    return 9


def select_coins(params: CoinSelectionParams) -> tuple[CoinSelection, str]:

    # Validate target value isn't something silly
    if params.target_value == 0 or params.target_value > MAX_MONEY:
        return (CoinSelection.invalid_spend(params), "failed") # type: ignore

    # Check for insufficient funds
    if params.total_value < params.target_value:
        return (CoinSelection.insufficient_funds(params), "failed") # type: ignore

    if params.total_effective_value < params.target_value + params.fixed_fee:
        return (CoinSelection.insufficient_funds_after_fees(params), "failed") # type: ignore

    # Return branch and bound selection (more optimized) if possible
    bnb_selection = select_coins_branch_and_bound(params)
    if bnb_selection.outcome == CoinSelection.Outcome.SUCCESS:
        return (bnb_selection, "bnb")
    # Otherwise return knapsack_selection (less optimized) if possible
    else:
        knapsack_selection = select_coins_knapsack_solver(params)
        if knapsack_selection.outcome == CoinSelection.Outcome.SUCCESS:
            return (knapsack_selection, "knapsack")
        else:
            # If all else fails, return single random draw selection (not optomized) as a fallback
            return (select_coins_single_random_draw(params), "srd")


class PythonCoinSelector:

    def __init__(self, long_term_feerate: float, dust_limit: int, input_drain_weight: int, output_drain_weight: int) -> None:
        self.deposit_count = 0
        self.candidates: list[OutputGroup] = []
        self.long_term_feerate = long_term_feerate
        self.dust_limit = dust_limit
        self.input_drain_weight = input_drain_weight
        self.output_drain_weight = output_drain_weight

    def values(self) -> list[int]:
        return [int(x.value) for x in self.candidates]

    def deposit(self, scenario_entry: dict) -> None:
        input_coin = InputCoin(
            tx_hash=str(self.deposit_count),
            vout=0,
            value=Value(scenario_entry.get("amount", 0)).value_sat,
            input_bytes=SEGWIT_V1_TXIN_WEIGHT
        )
        self.deposit_count += 1
        self.candidates.append(OutputGroup("", [input_coin]))

    def cost_to_empty_at_long_term_feerate(self) -> float:
        return sum((x.input_bytes for y in self.candidates for x in y.outputs)) * self.long_term_feerate

    def balance(self) -> int:
        return sum(self.values())

    def withdraw(self, pending_payments: list[dict], fee_rate_per_kvb: float) -> dict:
        target_value = sum((x.get("amount", 0) for x in pending_payments))
        payment_count = len(pending_payments)
        target_feerate = int(fee_rate_per_kvb * 1e5) # from btc per kvb to sat per vb
        output_weight_total = sum((x.get("weight", 0) for x in pending_payments))
        output_count = len(pending_payments)
        simulation_entry = {
            "id": 0,
            "inputs": [],
            "amount": target_value,
            "fee": 0,
            "target_feerate": target_feerate,
            "real_feerate": None,
            "algorithm": "failed",
            "negative_effective_valued_inputs": None,
            "output_count": None,
            "change_amount": None,
            "utxo_count_before_payment": len(self.candidates),
            "utxo_count_after_payment": len(self.candidates),
            "cost_to_empty_at_long_term_feerate": self.cost_to_empty_at_long_term_feerate(),
            "balance": self.balance(),
            "waste_score": None
        }

        base_weight = (
            4 # nVersion
            + 4 # nLockTime
            + varint_size(0) # inputs varint
            + varint_size(output_count) # outputs varint
            * 4
            + output_weight_total
        )
        selection_params = CoinSelectionParams(
           self.candidates,
           target_value,
           target_feerate,
           int(self.long_term_feerate),
           self.input_drain_weight,
           self.output_drain_weight,
           base_weight,
        )

        coin_selection, algorithm = select_coins(selection_params)

        if coin_selection.outcome != CoinSelection.Outcome.SUCCESS:
            return simulation_entry

        simulation_entry["algorithm"] = algorithm

        selected_value = sum((x.value for x in coin_selection.outputs))
        input_weight = sum((x.input_bytes for x in coin_selection.outputs))
        input_waste = input_weight * (target_feerate - self.long_term_feerate);

        simulation_entry["output_count"] = payment_count

        total_weight = input_weight + base_weight

        new_candidates = []
        if coin_selection.change_value > 0:
            simulation_entry["change_amount"] = coin_selection.change_value
            total_weight += SEGWIT_V1_TXOUT_WEIGHT
            simulation_entry["output_count"] += 1
            new_candidates.append(
                OutputGroup("", [
                    InputCoin(
                        tx_hash=str(self.deposit_count),
                        vout=0,
                        value=coin_selection.change_value,
                        input_bytes=SEGWIT_V1_TXIN_WEIGHT
            )]))
            self.deposit_count += 1

        fee = total_weight * target_feerate
        change_waste = selected_value - fee

        if coin_selection.change_value > 0:
            change_waste = self.long_term_feerate * SEGWIT_V1_TXIN_WEIGHT + target_feerate * SEGWIT_V1_TXOUT_WEIGHT

        simulation_entry["waste_score"] = input_waste + change_waste

        simulation_entry["real_feerate"] = None
        if total_weight != 0:
            simulation_entry["real_feerate"] =  coin_selection.fee / total_weight



        simulation_entry["inputs"] = [x.value for x in coin_selection.outputs]
        simulation_entry["negative_effective_valued_inputs"] = sum(1 for x in coin_selection.outputs if x.effective_value < 0)
        simulation_entry["fee"] = coin_selection.fee

        stxo = set((x.tx_hash for x in coin_selection.outputs))
        for x in self.candidates:
            new_inputs = []
            for y in x.outputs:
                if y.tx_hash in stxo:
                    continue
                new_inputs.append(y)

            if new_inputs:
                new_candidates.append(OutputGroup("", new_inputs))

        self.candidates = new_candidates

        simulation_entry["utxo_count_after_payment"] = len(self.candidates);

        return simulation_entry
