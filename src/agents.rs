use crate::market::{Order, Side, Price};
use crate::rng::DetRng;

pub trait Trader {
    fn id(&self) -> u32;
    /// Produce a quote (an order) given current rng. Integer-only.
    fn quote(&mut self, rng: &mut DetRng) -> Order;
}

/// Zero-Intelligence-Constrained baseline.
pub struct Zic { pub id: u32, pub side: Side, pub limit: Price, pub min_price: Price }
impl Trader for Zic {
    fn id(&self) -> u32 { self.id }
    fn quote(&mut self, rng: &mut DetRng) -> Order {
        let price = match self.side {
            Side::Bid => rng.gen_range_i64(self.min_price, self.limit),
            Side::Ask => rng.gen_range_i64(self.limit, self.min_price.max(self.limit) + 100),
        };
        Order { trader_id: self.id, side: self.side, price, qty: 1, seq: 0 }
    }
}

/// Evolvable integer-parameterised strategy (the optimisation target).
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub struct StrategyParams { pub aggressiveness: i64, pub spread: i64 } // bounded integers

#[derive(Clone)]
pub struct Evolvable { pub id: u32, pub side: Side, pub limit: Price, pub min_price: Price, pub params: StrategyParams }
impl Trader for Evolvable {
    fn id(&self) -> u32 { self.id }
    fn quote(&mut self, rng: &mut DetRng) -> Order {
        // deterministic price = limit adjusted by params + small seeded jitter
        let jitter = rng.gen_range_i64(0, self.params.spread.max(0) + 1);
        let price = match self.side {
            Side::Bid => (self.limit - self.params.aggressiveness - jitter).clamp(self.min_price, self.limit),
            Side::Ask => (self.limit + self.params.aggressiveness + jitter).max(self.limit),
        };
        Order { trader_id: self.id, side: self.side, price, qty: 1, seq: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::DetRng;
    #[test]
    fn zic_quotes_within_limit() {
        let mut z = Zic { id: 1, side: crate::market::Side::Bid, limit: 100, min_price: 1 };
        let mut r = DetRng::from_seed(3);
        for _ in 0..500 {
            let o = z.quote(&mut r);
            assert!(o.price >= 1 && o.price <= 100); // buyer never bids above limit
            assert_eq!(o.side, crate::market::Side::Bid);
        }
    }
    #[test]
    fn evolvable_is_pure_in_params_and_rng() {
        let p = StrategyParams { aggressiveness: 30, spread: 5 };
        let mut a = Evolvable { id: 9, side: crate::market::Side::Ask, limit: 100, min_price: 1, params: p };
        let mut b = a.clone();
        let mut r1 = DetRng::from_seed(8); let mut r2 = DetRng::from_seed(8);
        assert_eq!(a.quote(&mut r1).price, b.quote(&mut r2).price);
    }
}
