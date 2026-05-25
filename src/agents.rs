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

    // New: ZIC ask stays within its configured range [limit, min_price.max(limit) + 100].
    #[test]
    fn zic_ask_stays_within_range() {
        let limit = 50;
        let min_price = 1;
        let mut z = Zic { id: 2, side: Side::Ask, limit, min_price };
        let mut r = DetRng::from_seed(7);
        let lo = limit;
        let hi = min_price.max(limit) + 100;
        for _ in 0..500 {
            let o = z.quote(&mut r);
            assert_eq!(o.side, Side::Ask);
            assert!(o.price >= lo && o.price <= hi,
                "ZIC ask price {} out of [{}, {}]", o.price, lo, hi);
        }
    }

    // New: Evolvable bid clamps to [min_price, limit].
    #[test]
    fn evolvable_bid_clamps_to_range() {
        let limit = 100i64;
        let min_price = 10i64;
        let p = StrategyParams { aggressiveness: 30, spread: 5 };
        let mut trader = Evolvable { id: 3, side: Side::Bid, limit, min_price, params: p };
        let mut r = DetRng::from_seed(11);
        for _ in 0..500 {
            let o = trader.quote(&mut r);
            assert_eq!(o.side, Side::Bid);
            assert!(o.price >= min_price && o.price <= limit,
                "Evolvable bid price {} out of [{}, {}]", o.price, min_price, limit);
        }
    }

    // New: Evolvable ask always >= limit.
    #[test]
    fn evolvable_ask_stays_at_or_above_limit() {
        let limit = 60i64;
        let min_price = 1i64;
        let p = StrategyParams { aggressiveness: 20, spread: 8 };
        let mut trader = Evolvable { id: 4, side: Side::Ask, limit, min_price, params: p };
        let mut r = DetRng::from_seed(13);
        for _ in 0..500 {
            let o = trader.quote(&mut r);
            assert_eq!(o.side, Side::Ask);
            assert!(o.price >= limit,
                "Evolvable ask price {} must be >= limit {}", o.price, limit);
        }
    }

    // New: extreme StrategyParams still produce in-bounds, valid orders for both sides.
    #[test]
    fn extreme_strategy_params_stay_in_bounds() {
        let limit = 100i64;
        let min_price = 1i64;
        let extreme = StrategyParams { aggressiveness: 1_000_000, spread: 1_000_000 };
        let mut bid_trader = Evolvable { id: 5, side: Side::Bid, limit, min_price, params: extreme };
        let mut ask_trader = Evolvable { id: 6, side: Side::Ask, limit, min_price, params: extreme };
        let mut r = DetRng::from_seed(19);
        for _ in 0..200 {
            let bo = bid_trader.quote(&mut r);
            assert!(bo.price >= min_price && bo.price <= limit,
                "extreme bid price {} out of [{}, {}]", bo.price, min_price, limit);
            let ao = ask_trader.quote(&mut r);
            assert!(ao.price >= limit,
                "extreme ask price {} must be >= limit {}", ao.price, limit);
        }
    }
}
