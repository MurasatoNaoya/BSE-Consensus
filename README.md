# bse-consensus

Proof-of-Useful-Work consensus. Mining runs a deterministic optimisation over a simulated limit-order-book market. Producing a block requires the full run; verification re-checks a random sample of steps in `O(k·log n)`.

## Usage

```bash
cargo run -- mine --seed 7 --difficulty 2 | cargo run -- verify
cargo run -- mine --seed 7 --difficulty 2 | cargo run -- inspect
```

- `mine --seed <u64> --difficulty <u32>` — prints a block + proof as JSON.
- `verify` — reads the JSON on stdin; exit 0 if valid, 1 if not.
- `inspect` — reads the JSON on stdin; prints a summary.

## Mining

Given `(seed, difficulty)`:

1. Seed a ChaCha20 RNG; initialise a population of 8 `StrategyParams`.
2. Run `difficulty*4 + 4` generations of evolutionary search (tournament selection, elitism, integer mutation). Per-generation fitness is the integer profit the best strategy makes against ZIC traders in a market session.
3. Each generation is a `Frame`; the frame hashes form a BLAKE3 Merkle tree.
4. Block = `{seed, difficulty, n_frames, root, best_strategy, claimed_fitness}`, plus a Fiat-Shamir proof.

## Verification

Challenges derive from `block_commit = BLAKE3(seed ‖ difficulty ‖ n_frames ‖ root)`, so any header edit changes which steps must be answered. For each of `k` challenge indices (`H(block_commit ‖ i) mod (n_frames−1)`, with the last transition force-included):

- Merkle inclusion of `frame[c]` and `frame[c+1]`.
- `step(frame[c]) == frame[c+1]` (re-executed).

Also required: `n_frames == generations(difficulty)`; `claimed_fitness >= threshold(difficulty)`; `k >= min_challenges(difficulty) = 8 + difficulty`; per-frame `seed`/`gen` consistency; the final frame yields the block's `best_strategy` and `claimed_fitness`.

Tampering with any of the `n` transitions is detected with probability `1 − (1 − 1/n)^k`.

## Determinism

Same `(seed, difficulty)` yields the same commitment on any machine: integer-only arithmetic in hashed data, seeded ChaCha20 (stream position recorded per frame), `BTreeMap`/`Vec` (no `HashMap`), BLAKE3 with domain tags, little-endian frame encoding.

## Limitations

Single node; no P2P, no multi-block chain, no difficulty retargeting. A run is fully determined by its seed, so seeds should be bound to external randomness (e.g. a prior block hash) to prevent offline grinding. The market model is minimal — enough that fitness varies with strategy.

## Layout

```
src/
  rng.rs            seeded ChaCha20 with recordable position
  crypto.rs         BLAKE3 + Merkle tree & inclusion proofs
  agents.rs         ZIC and evolvable traders
  market/book.rs    integer limit-order book (price-time priority)
  market/session.rs deterministic session → integer fitness
  pouw.rs           Frame, Block, mine, step, prove, verify
  bin/bse-consensus.rs   CLI
tests/cli.rs        roundtrip, tamper-rejection, inspect
```

Rust 2021; `Cargo.lock` committed.
