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
