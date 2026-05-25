use crate::agents::StrategyParams;
use crate::crypto::{hash, MerkleTree};
use crate::market::run_session;
use crate::rng::DetRng;
use serde::{Serialize, Deserialize};

const POP: usize = 8;
const EVAL_SEED_BASE: u64 = 0xB5E; // fixed: session seed per generation derived from this + gen
const EVAL_STEPS: u32 = 200;

fn generations(difficulty: u32) -> usize { (difficulty as usize) * 4 + 4 }
pub fn threshold(difficulty: u32) -> i64 { (difficulty as i64) * 10 }

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub seed: u64,
    pub gen: u32,
    pub population: Vec<StrategyParams>, // length POP, ordered
    pub fitness: Vec<i64>,               // fitness of each, same order
    pub rng_pos: u128,                   // DetRng word position at generation start
}
impl Frame {
    /// Canonical little-endian serialisation → leaf hash input. Deterministic.
    pub fn canonical_bytes(&self) -> Vec<u8> { serde_json::to_vec(self).expect("frame serialises") }
    pub fn leaf(&self) -> [u8;32] { hash(&self.canonical_bytes()) }
    pub fn best(&self) -> (StrategyParams, i64) {
        let (i, &f) = self.fitness.iter().enumerate().max_by_key(|(_, &f)| f).unwrap();
        (self.population[i], f)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Block {
    pub seed: u64, pub difficulty: u32, pub n_frames: u32,
    pub root: [u8;32], pub best_strategy: StrategyParams, pub claimed_fitness: i64,
}

fn eval(seed: u64, gen: u32, p: StrategyParams) -> i64 {
    run_session(seed.wrapping_add(EVAL_SEED_BASE).wrapping_add(gen as u64), p, EVAL_STEPS)
}

fn initial_population(rng: &mut DetRng) -> Vec<StrategyParams> {
    (0..POP).map(|_| StrategyParams {
        aggressiveness: rng.gen_range_i64(0, 50),
        spread: rng.gen_range_i64(0, 10),
    }).collect()
}

fn first_frame(seed: u64) -> Frame {
    let mut rng = DetRng::from_seed(seed);
    let pos = rng.position();
    let population = initial_population(&mut rng);
    let fitness = population.iter().enumerate().map(|(_, &p)| eval(seed, 0, p)).collect();
    Frame { seed, gen: 0, population, fitness, rng_pos: pos }
}

/// Deterministic single-generation transition. Pure function of `frame`.
pub fn step(frame: &Frame) -> Frame {
    let mut rng = DetRng::from_seed(frame.seed);
    rng.set_position(frame.rng_pos);
    // advance rng past this generation's mutation draws to get next start pos AFTER producing children
    let mut next_pop = Vec::with_capacity(POP);
    // elitism: keep current best
    let (best_p, _best_f) = frame.best();
    next_pop.push(best_p);
    while next_pop.len() < POP {
        // tournament pick + mutate
        let i = rng.gen_range_i64(0, (POP-1) as i64) as usize;
        let j = rng.gen_range_i64(0, (POP-1) as i64) as usize;
        let parent = if frame.fitness[i] >= frame.fitness[j] { frame.population[i] } else { frame.population[j] };
        next_pop.push(StrategyParams {
            aggressiveness: (parent.aggressiveness + rng.gen_range_i64(-5, 5)).clamp(0, 50),
            spread: (parent.spread + rng.gen_range_i64(-2, 2)).clamp(0, 10),
        });
    }
    let next_pos = rng.position();
    let gen = frame.gen + 1;
    let fitness = next_pop.iter().map(|&p| eval(frame.seed, gen, p)).collect();
    Frame { seed: frame.seed, gen, population: next_pop, fitness, rng_pos: next_pos }
}

/// Mine: run the deterministic optimisation, commit the trajectory.
pub fn mine(seed: u64, difficulty: u32) -> (Block, Vec<Frame>) {
    let n = generations(difficulty);
    let mut frames = Vec::with_capacity(n);
    frames.push(first_frame(seed));
    for _ in 1..n { let nxt = step(frames.last().unwrap()); frames.push(nxt); }
    let leaves: Vec<[u8;32]> = frames.iter().map(|f| f.leaf()).collect();
    let root = MerkleTree::build(&leaves).root();
    let (best_strategy, claimed_fitness) = frames.last().unwrap().best();
    (Block { seed, difficulty, n_frames: n as u32, root, best_strategy, claimed_fitness }, frames)
}

use crate::crypto::{MerkleProof, verify_proof};

/// Fiat-Shamir challenge indices in `[0, n_frames-1)` (each indexes a transition
/// `f[c] -> f[c+1]`), derived from the committed root.
///
/// The final transition `n_frames-2` is FORCE-INCLUDED as the first index so the
/// verifier's final-frame fitness-claim consistency check always runs. The
/// remaining `k-1` indices are derived via Fiat-Shamir: `H(root ‖ i) mod (n_frames-1)`.
pub fn challenges(root: &[u8; 32], n_frames: u32, k: u32) -> Vec<u32> {
    let span = (n_frames - 1) as u64; // number of transitions
    let mut out = Vec::with_capacity(k as usize);
    if k == 0 { return out; }
    // force-include the final transition first
    out.push(n_frames - 2);
    // derive the remaining k-1 indices via Fiat-Shamir
    for i in 0..(k - 1) {
        let mut buf = [0u8; 36];
        buf[..32].copy_from_slice(root);
        buf[32..].copy_from_slice(&i.to_le_bytes());
        let h = hash(&buf);
        let v = u64::from_le_bytes(h[..8].try_into().unwrap());
        out.push((v % span) as u32);
    }
    out
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChallengeAnswer {
    pub index: u32,
    pub frame: Frame,
    pub frame_next: Frame,
    pub path: MerkleProof,
    pub path_next: MerkleProof,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Proof {
    pub answers: Vec<ChallengeAnswer>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError {
    MalformedBlock,
    MerklePathInvalid,
    TransitionMismatch(u32),
    BelowThreshold,
    FitnessClaimInconsistent,
}

/// Build a spot-check proof: answer each Fiat-Shamir challenge with both frames of
/// the challenged transition and their Merkle inclusion paths against the committed root.
pub fn prove(block: &Block, frames: &[Frame], k: u32) -> Proof {
    let leaves: Vec<[u8; 32]> = frames.iter().map(|f| f.leaf()).collect();
    let tree = MerkleTree::build(&leaves);
    let answers = challenges(&block.root, block.n_frames, k)
        .into_iter()
        .map(|c| {
            let i = c as usize;
            ChallengeAnswer {
                index: c,
                frame: frames[i].clone(),
                frame_next: frames[i + 1].clone(),
                path: tree.proof(i),
                path_next: tree.proof(i + 1),
            }
        })
        .collect();
    Proof { answers }
}

/// Cheap, sound verification. Nothing miner-claimed that the verifier can recompute is trusted.
pub fn verify(block: &Block, proof: &Proof, k: u32) -> Result<(), VerifyError> {
    if block.n_frames < 2 { return Err(VerifyError::MalformedBlock); }
    // usefulness enforced by the verifier
    if block.claimed_fitness < threshold(block.difficulty) { return Err(VerifyError::BelowThreshold); }
    // recompute the challenges from the root — the prover cannot choose them
    let expected = challenges(&block.root, block.n_frames, k);
    if proof.answers.len() != expected.len() { return Err(VerifyError::MalformedBlock); }
    for (ans, &exp) in proof.answers.iter().zip(expected.iter()) {
        if ans.index != exp { return Err(VerifyError::MalformedBlock); }
        // Merkle inclusion of BOTH frames against the committed root
        if !verify_proof(&block.root, &ans.frame.leaf(), &ans.path) { return Err(VerifyError::MerklePathInvalid); }
        if !verify_proof(&block.root, &ans.frame_next.leaf(), &ans.path_next) { return Err(VerifyError::MerklePathInvalid); }
        // re-execute the single transition deterministically — recomputed, not trusted
        if step(&ans.frame) != ans.frame_next { return Err(VerifyError::TransitionMismatch(ans.index)); }
        // when this challenge lands on the last transition, the claimed best must match
        if ans.frame_next.gen + 1 == block.n_frames {
            let (bp, bf) = ans.frame_next.best();
            if bp != block.best_strategy || bf != block.claimed_fitness {
                return Err(VerifyError::FitnessClaimInconsistent);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mining_is_deterministic() {
        let (b1, _) = mine(7, 3);
        let (b2, _) = mine(7, 3);
        assert_eq!(b1.root, b2.root);
        assert_eq!(b1.claimed_fitness, b2.claimed_fitness);
        assert_ne!(mine(8, 3).0.root, b1.root);
    }
    #[test]
    fn step_reproduces_recorded_frame() {
        let (_b, frames) = mine(7, 3);
        for i in 0..frames.len()-1 {
            assert_eq!(step(&frames[i]), frames[i+1], "transition {i} must reproduce");
        }
    }
}

#[cfg(test)]
mod verify_tests {
    use super::*;
    #[test]
    fn challenges_are_deterministic_and_in_range() {
        let (b, _) = mine(7, 3);
        let c1 = challenges(&b.root, b.n_frames, 4);
        let c2 = challenges(&b.root, b.n_frames, 4);
        assert_eq!(c1, c2);
        // every challenged transition index must be < n_frames-1
        for c in &c1 { assert!(*c < b.n_frames - 1); }
        // the final transition n_frames-2 is force-included as the first index
        assert_eq!(c1[0], b.n_frames - 2);
    }
    #[test]
    fn valid_block_verifies() {
        let (b, frames) = mine(7, 3);
        let proof = prove(&b, &frames, 4);
        assert!(verify(&b, &proof, 4).is_ok());
    }
    #[test]
    fn tampered_fitness_below_threshold_rejected() {
        let (mut b, frames) = mine(7, 1);
        let proof = prove(&b, &frames, 4);
        b.claimed_fitness = threshold(1) - 1; // claim below threshold
        assert!(matches!(verify(&b, &proof, 4), Err(VerifyError::BelowThreshold)));
    }
    #[test]
    fn tampered_frame_rejected() {
        let (b, frames) = mine(7, 3);
        let mut proof = prove(&b, &frames, 4);
        // corrupt the first challenged "next" frame so step() != next
        if let Some(ca) = proof.answers.get_mut(0) { ca.frame_next.fitness[0] ^= 0x1234; }
        assert!(verify(&b, &proof, 4).is_err());
    }
}
