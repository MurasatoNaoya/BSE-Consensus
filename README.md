# bse-consensus

**Proof-of-Useful-Work consensus — deterministic market-simulation mining with cheap, sound verification.**

---

## The idea

Proof-of-Work wastes computation by design: the only point of the SHA256 loop is to burn cycles. PoUW redirects that effort. Here the "useful work" is a deterministic evolutionary optimisation of an integer-parameterised trading strategy inside a simulated limit-order-book market — a derivative of the Bristol Stock Exchange (BSE). A miner must run the full evolutionary search to produce a valid block; a verifier can spot-check any subset of transitions cheaply via Merkle inclusion proofs and re-execution. The simulation output (the best trained strategy) is the payload.

---

## How mining works

Given `seed` (u64) and `difficulty` (u32):

1. Derive a population of 8 `StrategyParams` (aggressiveness + spread, both bounded integers) from a seeded ChaCha20 RNG.
2. Run `generations(difficulty) = difficulty × 4 + 4` generations of an evolutionary search. Each generation is one **`Frame`**: tournament selection, elitism, mutation (integer arithmetic only), then fitness evaluation — integer profit the best strategy earns trading against Zero-Intelligence-Constrained (ZIC) counter-traders in `run_session`.
3. Compute a BLAKE3 Merkle tree over canonical little-endian frame hashes. The tree root is the trajectory commitment.
4. Emit a **`Block`**: `{seed, difficulty, n_frames, root, best_strategy, claimed_fitness}`.

The block also carries a **`Proof`**: Fiat-Shamir spot-check answers (frame pairs + Merkle paths) at `k ≥ min_challenges(difficulty)` positions.

---

## How verification works (cheap + sound)

The verifier re-derives everything from the header; nothing miner-claimed is trusted.

**Header-bound commitment.** `block_commit = BLAKE3(seed_le ‖ difficulty_le ‖ n_frames_le ‖ root)`. Challenges are derived from `block_commit`, not the bare root. Any edit to `seed`, `difficulty`, `n_frames`, or `root` changes the commitment and therefore the expected challenge set — so relabelling or truncating a block changes which transitions must be answered, and the presented proof no longer matches.

**Challenge derivation.** `k` indices in `[0, n_frames−1)` are derived via Fiat-Shamir (`H(commit ‖ i) mod (n_frames−1)`). The final transition (`n_frames−2`) is force-included as index 0, so the last-frame fitness claim is always checked.

**Per-challenge checks:**
- Merkle inclusion of both `frame[c]` and `frame[c+1]` against the committed `root`.
- Re-execution: `step(frame[c])` must equal `frame[c+1]` exactly (deterministic, recomputed by the verifier).
- Per-frame structural consistency: `frame.seed == block.seed`, `frame.gen == c`.
- On the final transition: `best(frame[n−1])` must match `block.best_strategy` and `block.claimed_fitness`.

**Enforced checks:**
- `block.n_frames == generations(block.difficulty)` — a truncated prefix cannot pass as the full run.
- `block.claimed_fitness ≥ threshold(difficulty)` — usefulness enforced by the verifier, not miner-trusted.
- `k ≥ min_challenges(difficulty) = 8 + difficulty` — the verifier rejects an attacker-chosen tiny `k`.

**Cost:** `O(k · log n)` for the verifier vs `O(n)` to mine.

---

## Determinism strategy

| Concern | Mechanism |
|---------|-----------|
| RNG | Seeded ChaCha20 (`DetRng`), stream position (`get_word_pos`) recorded per `Frame` |
| Arithmetic | Integer only in anything hashed — no floats |
| Collections | `BTreeMap` + `Vec` throughout; no `HashMap` |
| Order book | FIFO price-time priority, all prices `i64` |
| Frame encoding | Fixed-field-order little-endian bytes; `POP` (= 8) is a protocol constant |
| Hash | BLAKE3 with domain tags (0x00 leaf, 0x01 node) |

Same `seed` + `difficulty` produces an identical `root` on any machine.

---

## Soundness and limitations

**Probabilistic soundness.** A prover that falsifies any one of the `n` transitions is caught with probability ≈ `1 − (1 − 1/n)^k`. With `k = min_challenges(difficulty) = 8 + difficulty` and `n = difficulty × 4 + 4`, soundness improves with difficulty. The `min_challenges` floor prevents a proof with a single spot-check from slipping through.

**Residual caveat — seed grinding.** Because the entire run is deterministic from `seed`, a miner can try many seeds offline to fish for one that yields a favourable trajectory (high fitness, or coincidentally few challenged transitions that are easy to fake). This is inherent in any seed-based PoUW and is documented rather than hidden. A production system would need external randomness for seed selection (e.g. committed to a previous block hash).

**Scope.** This is a single-node proof of concept. There is no P2P network, no linked multi-block chain, and no difficulty retargeting (all deferred). The integer market model is intentionally simplified compared to a real exchange — it is sufficient to make fitness vary meaningfully with `StrategyParams`, which is the design goal.

---

## Differences from v0.1 (the dissertation PoC)

The original Python prototype is preserved at tag `v0.1.0` and in `legacy/`.

| | v0.1 PoC | This rewrite (v1.0.0) |
|---|---|---|
| Trajectory coverage | Verification covered day 1 only | Fiat-Shamir spot-checks cover the whole trajectory |
| Usefulness threshold | Miner-reported, not enforced | Validator-enforced: `claimed_fitness ≥ threshold(difficulty)` |
| Cryptographic binding | Bare scalar commitment | BLAKE3 Merkle root + header-bound `block_commit` |
| Relabelling resistance | Not addressed | Any header edit changes `block_commit` → challenge set mismatch |
| Implementation | Python (NumPy / BSE.py) | Rust, integer-only, `Cargo.lock` committed |

---

## Quick start

```bash
# Mine a block at seed=7, difficulty=2; pipe straight to verify
cargo run -- mine --seed 7 --difficulty 2 | cargo run -- verify
# → VALID: fitness N >= threshold, all spot-checks pass

# Human-readable summary
cargo run -- mine --seed 7 --difficulty 2 | cargo run -- inspect
# → seed=7 difficulty=2 frames=12 challenges=12
#   best=StrategyParams { aggressiveness: ..., spread: ... } fitness=...
#   root=<hex>

# Higher difficulty (more generations, more challenges required)
cargo run --release -- mine --seed 42 --difficulty 5 | cargo run --release -- verify
```

Local toolchain: Rust edition 2021; pinned dependencies (see `Cargo.lock`, which is committed for reproducible builds).

---

## Module layout

```
src/
  rng.rs          — DetRng: seeded ChaCha20 with recordable stream position
  crypto.rs       — BLAKE3 helpers + domain-separated Merkle tree + inclusion proofs
  agents.rs       — Zic (Zero-Intelligence-Constrained) and Evolvable (StrategyParams) traders
  market/
    book.rs       — Integer limit-order book (BTreeMap, price-time priority)
    session.rs    — run_session: deterministic market session → integer fitness
  pouw.rs         — Frame, Block, mine, step, prove, verify, block_commit, challenges
  bin/
    bse-consensus.rs  — CLI: mine / verify / inspect subcommands

legacy/           — Original Python dissertation prototype (BSE.py, notebooks)
tests/cli.rs      — Integration tests: roundtrip, tamper rejection, inspect
```
