# Phase 0: Solana fundamentals

Accounts, PDAs, instructions, rent/ownership, Anchor overview. Front-loaded on purpose — everything downstream depends on this being solid, not fast.

# Phase 1: Rewrite the core Rust/Anchor logic (deliberate, not fast)

- Re-derive the account structs (bet state, PDA, escrow) — from a blank file, not copy-paste
- Rewrite `create_bet` — define validation checks as you go (valid amount, valid opponent, etc.)
- Rewrite `accept_bet` — enforce who's allowed to accept, prevent double-accept
- Rewrite `resolve_bet`/payout with the oracle problem front and center — explicitly define: who/what is allowed to assert the real-world outcome, how that assertion is validated before funds move, and what happens if that source is wrong, late, or contested. Treat this as the most important guard in the whole contract, not a formality.
- Leave scaffolding alone (project setup, wallet connection, frontend)
- Narrate each check out loud as you write it: why does this guard exist, what does it prevent

# Phase 2: Verification layer (interviewers will probe hardest)

- Happy-path tests (create → accept → resolve → payout)
- Edge cases: double-claim, wrong-signer, early/expired claim, underfunded escrow
- Oracle/resolution edge cases specifically: resolver tries to submit after deadline, resolver tries to submit twice, unauthorized account tries to resolve, resolution submitted with contradictory/malformed outcome data
- One sentence per test on why that failure mode matters — this becomes your spoken answer

# Phase 3: C++ offline validator (independent verification tool)

- C++ struct mirroring your bet account
- Export path — dump account state to JSON from your Rust/TS tooling
- JSON parser
- Validation functions for your invariants (balance matches wager, legal status transitions, deadline respected, no overflow/negative amounts, resolution source was authorized)
- Output pass/fail with specific failure reason
- 4 test cases: valid bet, tampered balance, illegal status jump, expired bet
