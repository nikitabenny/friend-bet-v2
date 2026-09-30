# friend-bet-v2

An Anchor program for peer-to-peer bets between friends: a creator stakes
funds into an escrow vault, a counterparty accepts by matching a side, a
designated resolver names the outcome, and the vault pays out the winning
side. Written from scratch (not copied from the v1 version of this project)
as a deliberate exercise in Solana/Anchor fundamentals — see [phases.md](phases.md)
for the plan this repo follows.

## Structure

```
programs/friend-bet-v2/   Anchor program (Rust)
tests/                    Anchor/mocha integration tests (TypeScript)
validator/                offline C++ tool that independently re-checks
                           on-chain invariants from a JSON snapshot — see
                           validator/README.md
```

## Program

Instructions implemented in [lib.rs](programs/friend-bet-v2/src/lib.rs):

| Instruction | Does |
|---|---|
| `create_bet` | creates a `CreatorProfile` (if needed), a `BetAccount`, its `Vault`, and the creator's own `Participant` stake |
| `accept_bet` | a counterparty joins a side, matching stake into the vault |
| `resolve_bet` | the designated resolver names the winning side |
| `payout` | pays a winning `Participant` their share of the vault |

PDA seeds:

| Account | Seeds |
|---|---|
| `CreatorProfile` | `[b"creator", creator]` |
| `BetAccount` | `[b"bet", creator, bet_id_le_bytes]` |
| `Vault` | `[b"vault", creator, bet_id_le_bytes]` |
| `Participant` | `[b"particip", owner, bet_id_le_bytes]` |

`bet_id` is `CreatorProfile.next_bet_id` at the moment a bet is created —
it's per-creator state, not derivable from the creator's pubkey alone.

`cancel_bet` has an `Accounts` struct defined but no handler yet — in
progress.

## Dev commands

```
anchor build         # build the program
anchor test           # run the local validator + tests/*.ts
```
