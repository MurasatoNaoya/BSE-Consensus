# bse-consensus

**Proof-of-Useful-Work consensus.** The mining work is a deterministic optimisation run inside a simulated limit-order-book market — not arbitrary hashing. Producing a block requires the full run; verifying one is cheap and cryptographically sound.

```bash
# Mine a block, pipe it straight to the verifier
cargo run -- mine --seed 7 --difficulty 2 | cargo run -- verify
# → VALID: fitness N >= threshold, all spot-checks pass

cargo run -- mine --seed 7 --difficulty 2 | cargo run -- inspect   # human-readable summary
```

## What the work is

Mining evolves an integer-parameterised trading strategy and reports the best one found:

1. Seed a ChaCha20 RNG from `(seed, difficulty)`; initialise a population of 8 `StrategyParams`.
2. Run `difficulty × 4 + 4` generations of an evolutionary search (tournament selection, elitism, integer mutation). Each generation's fitness is the integer profit the best strategy earns against zero-intelligence counter-traders in a deterministic market session.
3. Each generation is a `Frame`; their BLAKE3 hashes form a Merkle tree.
4. The block is `{seed, difficulty, n_frames, root, best_strategy, claimed_fitness}`, plus a Fiat-Shamir proof.

## Verification — cheap and sound

`O(k · log n)` to verify versus `O(n)` to mine. The verifier trusts nothing the miner claims.

- **Header-bound commitment.** `block_commit = BLAKE3(seed ‖ difficulty ‖ n_frames ‖ root)`. Challenges derive from this, so relabelling or truncating a block changes the challenge set and the proof no longer matches.
- **Spot-checks.** `k` challenge indices come from `H(block_commit ‖ i) mod (n_frames−1)`, with the final transition force-included. For each, the verifier checks Merkle inclusion of `frame[c]` and `frame[c+1]`, then **re-executes that single transition** and requires `step(frame[c]) == frame[c+1]`.
- **Enforced invariants.** `n_frames == generations(difficulty)`; `claimed_fitness ≥ threshold(difficulty)`; `k ≥ min_challenges(difficulty) = 8 + difficulty`; per-frame `seed`/`gen` consistency; the final frame must yield the block's `best_strategy` and `claimed_fitness`.

Falsifying any one of the `n` transitions is caught with probability `≈ 1 − (1 − 1/n)^k`.

## Determinism

Same `(seed, difficulty)` ⇒ identical commitment on any machine.

| Concern | Mechanism |
|---|---|
| RNG | Seeded ChaCha20; stream position recorded per `Frame` |
| Arithmetic | Integers only in anything hashed — no floats |
| Collections | `BTreeMap` / `Vec`; never `HashMap` |
| Hashing | BLAKE3 with domain tags (`0x00` leaf, `0x01` node) |
| Encoding | Fixed-field little-endian frame bytes |

## Limitations

Single-node proof of concept: no P2P network, no multi-block chain, no difficulty retargeting. Because a run is fully determined by its seed, a miner can grind seeds offline to fish for favourable outcomes — a production deployment would bind seeds to external randomness (e.g. a prior block hash).

## Layout

```
src/
  rng.rs            DetRng — seeded ChaCha20 with recordable position
  crypto.rs         BLAKE3 + domain-separated Merkle tree & proofs
  agents.rs         ZIC and evolvable traders
  market/
    book.rs         integer limit-order book (price-time priority)
    session.rs      deterministic session → integer fitness
  pouw.rs           Frame, Block, mine, step, prove, verify
  bin/bse-consensus.rs   CLI: mine / verify / inspect
tests/cli.rs        roundtrip, tamper-rejection, inspect
```

Rust 2021, integer-only core, `Cargo.lock` committed for reproducible builds.
