# validate

Offline invariant validator for the friend-bet-v2 escrow program.

It reads a JSON snapshot of on-chain account state (a bet, its vault, and
participants) and independently re-derives the program's invariants — it
shares no code with the Anchor program and doesn't trust the program's own
view of itself. Its only job is to disagree.

## Build

```
make
```

Requires a C++17 compiler. JSON parsing uses `nlohmann/json`, vendored as a
single header in `vendor/nlohmann/json.hpp` (no package manager needed).

## Run

```
./validate <snapshot.json>
```

Exit codes:

| Code | Meaning |
|------|---------|
| 0    | all invariants hold |
| 1    | at least one invariant violated |
| 2    | malformed input (bad JSON, missing/mistyped field) |

## Invariants checked

| ID | Check |
|----|-------|
| I1 | `vault.amount == sum(participant.stake)` |
| I2 | `sum(side_totals) == vault.amount` |
| I3 | `status == Resolved => winning_choice in [0, sides)` |
| I4 | status is consistent with participant count and outcome |
| I5 | `status == Resolved => dump_time > deadline` (settlement can't precede the event) |
| I6 | every field is within its declared domain |
| I7 | vault lamports actually cover all outstanding obligations |

I1/I2 check the program's bookkeeping against itself; I7 checks it against
real lamports, which is the one that matters most.

## Snapshot format

```jsonc
{
  "dump_time": 1754500000,
  "bet": {
    "address": "...", "id": 0, "creator": "...", "resolver": "...",
    "init_stake": 1000000000, "deadline": 1754600000, "status": "Accepted",
    "sides": 2, "winning_choice": null, "side_totals": [0, 0, 0, 0, 0]
  },
  "vault": {
    "id": 0, "creator": "...", "amount": 3000000000,
    "lamports": 3000000000, "rent_exempt_min": 890880
  },
  "participants": [
    { "owner": "...", "bet": "...", "stake": 1000000000, "choice": 0, "claimed": false }
  ]
}
```

`status` is one of `Created | Accepted | Resolved | Complete | Cancelled`.
`winning_choice` is an integer once resolved, `null` otherwise.

## Fixtures

```
make test
```

Runs every snapshot in `fixtures/` through `validate` and checks its exit
code against the expectation declared in `run_fixtures.sh`. Add a case there
when adding a new fixture — a fixture without a declared expectation is
skipped, not passed.
