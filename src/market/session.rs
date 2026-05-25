use crate::agents::{Evolvable, Zic, Trader, StrategyParams};
use crate::market::book::{OrderBook, Side};
use crate::rng::DetRng;

/// Run one deterministic market session; return the evolvable trader's integer fitness (profit).
///
/// The hero (Evolvable, id 0) is a Bid trader with limit 100. ZIC sellers (ids 1 and 3) have
/// limit 1 so they quote asks in [1, 101], which the hero can cross. ZIC buyers (ids 2 and 4)
/// provide liquidity on the other side. This ensures fitness genuinely varies with params:
/// higher aggressiveness → hero bids lower → fewer/cheaper trades → different profit curve.
pub fn run_session(seed: u64, params: StrategyParams, steps: u32) -> i64 {
    let mut rng = DetRng::from_seed(seed);
    let mut book = OrderBook::new();
    let mut hero = Evolvable { id: 0, side: Side::Bid, limit: 100, min_price: 1, params };
    // ZIC sellers (ids 1, 3) quote asks in [1, 101] — crossable with hero's bid
    // ZIC buyers (ids 2, 4) provide opposing liquidity
    let mut zics: Vec<Zic> = (1..=4u32).map(|i| Zic {
        id: i,
        side: if i % 2 == 1 { Side::Ask } else { Side::Bid },
        limit: 1,      // sellers: asks from [1, 101]; buyers: bids from [1, 1]
        min_price: 1,
    }).collect();
    let mut fitness: i64 = 0;
    let mut seq: u64 = 0;
    for _ in 0..steps {
        // fixed action order: hero (id 0) then zics in id order
        let mut q = hero.quote(&mut rng); q.seq = seq; seq += 1;
        for t in book.submit(q) {
            if t.buyer == 0 { fitness += hero.limit - t.price; }
            if t.seller == 0 { fitness += t.price - hero.limit; }
        }
        for z in zics.iter_mut() {
            let mut o = z.quote(&mut rng); o.seq = seq; seq += 1;
            for t in book.submit(o) {
                if t.buyer == 0 { fitness += hero.limit - t.price; }
                if t.seller == 0 { fitness += t.price - hero.limit; }
            }
        }
    }
    fitness
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::StrategyParams;
    #[test]
    fn session_is_reproducible() {
        let p = StrategyParams { aggressiveness: 20, spread: 4 };
        let f1 = run_session(123, p, 200);
        let f2 = run_session(123, p, 200);
        assert_eq!(f1, f2);                 // identical fitness
        assert_ne!(run_session(124, p, 200), f1); // different seed differs
    }
    #[test]
    fn better_params_can_score_higher() {
        // sanity: fitness responds to params (not constant)
        let a = run_session(5, StrategyParams{aggressiveness:0, spread:0}, 200);
        let b = run_session(5, StrategyParams{aggressiveness:40, spread:0}, 200);
        assert!(a != b);
    }

    // New: zero steps → zero fitness (no trades can occur).
    #[test]
    fn zero_steps_yields_zero_fitness() {
        let p = StrategyParams { aggressiveness: 10, spread: 2 };
        assert_eq!(run_session(42, p, 0), 0, "no steps means no trades, fitness must be 0");
    }

    // New: fitness is deterministic even when called after other unrelated work.
    #[test]
    fn fitness_deterministic_independent_of_intermediate_work() {
        let p = StrategyParams { aggressiveness: 15, spread: 3 };
        let f1 = run_session(77, p, 100);
        // do some unrelated work in between
        let _noise: Vec<i64> = (0..10)
            .map(|i| run_session(i, StrategyParams { aggressiveness: i as i64, spread: 0 }, 50))
            .collect();
        let f2 = run_session(77, p, 100);
        assert_eq!(f1, f2, "fitness must be deterministic regardless of intermediate calls");
    }

    // New: distinct param sets produce distinct, stable fitness values.
    #[test]
    fn distinct_params_produce_distinct_stable_fitness() {
        let p1 = StrategyParams { aggressiveness: 5, spread: 0 };
        let p2 = StrategyParams { aggressiveness: 25, spread: 5 };
        let f1a = run_session(99, p1, 150);
        let f1b = run_session(99, p1, 150);
        let f2a = run_session(99, p2, 150);
        let f2b = run_session(99, p2, 150);
        assert_eq!(f1a, f1b, "p1 fitness must be stable across calls");
        assert_eq!(f2a, f2b, "p2 fitness must be stable across calls");
        assert_ne!(f1a, f2a, "different params must yield different fitness values");
    }
}
