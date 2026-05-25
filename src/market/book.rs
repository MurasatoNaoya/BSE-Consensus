use std::collections::{BTreeMap, VecDeque};

pub type Price = i64;
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum Side { Bid, Ask }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Order { pub trader_id: u32, pub side: Side, pub price: Price, pub qty: u32, pub seq: u64 }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Trade { pub price: Price, pub qty: u32, pub buyer: u32, pub seller: u32 }

#[derive(Default)]
pub struct OrderBook { bids: BTreeMap<Price, VecDeque<Order>>, asks: BTreeMap<Price, VecDeque<Order>> }

impl OrderBook {
    pub fn new() -> Self { Self::default() }
    pub fn best_bid(&self) -> Option<Price> { self.bids.keys().next_back().copied() }
    pub fn best_ask(&self) -> Option<Price> { self.asks.keys().next().copied() }

    /// Submit an order; returns trades generated (taker crosses the book, remainder rests).
    pub fn submit(&mut self, mut incoming: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        loop {
            if incoming.qty == 0 { break; }
            // find best opposing level that crosses
            let cross_price = match incoming.side {
                Side::Bid => self.asks.keys().next().copied().filter(|&a| a <= incoming.price),
                Side::Ask => self.bids.keys().next_back().copied().filter(|&b| b >= incoming.price),
            };
            let Some(px) = cross_price else { break };
            let book = if incoming.side == Side::Bid { &mut self.asks } else { &mut self.bids };
            let level = book.get_mut(&px).unwrap();
            let resting = level.front_mut().unwrap();
            let q = incoming.qty.min(resting.qty);
            let (buyer, seller) = match incoming.side {
                Side::Bid => (incoming.trader_id, resting.trader_id),
                Side::Ask => (resting.trader_id, incoming.trader_id),
            };
            trades.push(Trade { price: px, qty: q, buyer, seller });
            incoming.qty -= q; resting.qty -= q;
            if resting.qty == 0 { level.pop_front(); if level.is_empty() { book.remove(&px); } }
        }
        if incoming.qty > 0 {
            let book = if incoming.side == Side::Bid { &mut self.bids } else { &mut self.asks };
            book.entry(incoming.price).or_default().push_back(incoming);
        }
        trades
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crossing_orders_match_at_resting_price() {
        let mut b = OrderBook::new();
        assert!(b.submit(Order{trader_id:1, side:Side::Ask, price:100, qty:5, seq:0}).is_empty());
        let trades = b.submit(Order{trader_id:2, side:Side::Bid, price:105, qty:3, seq:1});
        assert_eq!(trades, vec![Trade{price:100, qty:3, buyer:2, seller:1}]); // trade at resting ask
        assert_eq!(b.best_ask(), Some(100)); // 2 left resting
    }
    #[test]
    fn price_time_priority() {
        let mut b = OrderBook::new();
        b.submit(Order{trader_id:1, side:Side::Bid, price:100, qty:1, seq:0});
        b.submit(Order{trader_id:2, side:Side::Bid, price:100, qty:1, seq:1});
        let t = b.submit(Order{trader_id:3, side:Side::Ask, price:100, qty:1, seq:2});
        assert_eq!(t, vec![Trade{price:100, qty:1, buyer:1, seller:3}]); // earliest bid filled first
    }
    #[test]
    fn no_cross_rests() {
        let mut b = OrderBook::new();
        assert!(b.submit(Order{trader_id:1, side:Side::Bid, price:90, qty:1, seq:0}).is_empty());
        assert!(b.submit(Order{trader_id:2, side:Side::Ask, price:100, qty:1, seq:1}).is_empty());
        assert_eq!((b.best_bid(), b.best_ask()), (Some(90), Some(100)));
    }

    // New: a taker that sweeps multiple price levels in one order produces multiple trades
    // at the correct prices (each resting order's price).
    #[test]
    fn taker_sweeps_multiple_price_levels() {
        let mut b = OrderBook::new();
        // rest three asks at different prices
        b.submit(Order{trader_id:1, side:Side::Ask, price:100, qty:2, seq:0});
        b.submit(Order{trader_id:2, side:Side::Ask, price:102, qty:2, seq:1});
        b.submit(Order{trader_id:3, side:Side::Ask, price:104, qty:2, seq:2});
        // aggressive bid that crosses all three levels
        let trades = b.submit(Order{trader_id:10, side:Side::Bid, price:110, qty:6, seq:3});
        assert_eq!(trades.len(), 3, "must produce three separate trades");
        assert_eq!(trades[0].price, 100); // matched at lowest ask first
        assert_eq!(trades[1].price, 102);
        assert_eq!(trades[2].price, 104);
        assert_eq!(trades[0].qty + trades[1].qty + trades[2].qty, 6);
        // book should now be empty on ask side
        assert_eq!(b.best_ask(), None);
    }

    // New: partial fill leaves correct resting quantity.
    #[test]
    fn partial_fill_leaves_correct_resting_qty() {
        let mut b = OrderBook::new();
        b.submit(Order{trader_id:1, side:Side::Ask, price:100, qty:5, seq:0});
        // taker only wants 3
        let trades = b.submit(Order{trader_id:2, side:Side::Bid, price:105, qty:3, seq:1});
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].qty, 3);
        // ask at 100 should still exist with 2 remaining
        assert_eq!(b.best_ask(), Some(100));
        // confirm the remaining qty by fully consuming it
        let trades2 = b.submit(Order{trader_id:3, side:Side::Bid, price:100, qty:2, seq:2});
        assert_eq!(trades2.len(), 1);
        assert_eq!(trades2[0].qty, 2);
        assert_eq!(b.best_ask(), None);
    }

    // New: best_bid / best_ask update correctly after a level is fully consumed.
    #[test]
    fn best_bid_ask_update_after_level_consumed() {
        let mut b = OrderBook::new();
        b.submit(Order{trader_id:1, side:Side::Bid, price:95, qty:1, seq:0});
        b.submit(Order{trader_id:2, side:Side::Bid, price:90, qty:1, seq:1});
        b.submit(Order{trader_id:3, side:Side::Ask, price:100, qty:1, seq:2});
        b.submit(Order{trader_id:4, side:Side::Ask, price:105, qty:1, seq:3});
        assert_eq!(b.best_bid(), Some(95));
        assert_eq!(b.best_ask(), Some(100));
        // sell into best bid — consumes the 95 level entirely
        b.submit(Order{trader_id:5, side:Side::Ask, price:95, qty:1, seq:4});
        assert_eq!(b.best_bid(), Some(90), "best bid must fall to 90 after 95 level consumed");
        // buy into best ask — consumes the 100 level entirely
        b.submit(Order{trader_id:6, side:Side::Bid, price:100, qty:1, seq:5});
        assert_eq!(b.best_ask(), Some(105), "best ask must rise to 105 after 100 level consumed");
    }

    // New: an order that partially crosses then rests the unfilled remainder.
    #[test]
    fn partial_cross_then_rest() {
        let mut b = OrderBook::new();
        // one ask resting at 100 for qty 2
        b.submit(Order{trader_id:1, side:Side::Ask, price:100, qty:2, seq:0});
        // incoming bid that crosses qty 2 but has qty 5 — remainder rests at its limit
        let trades = b.submit(Order{trader_id:2, side:Side::Bid, price:100, qty:5, seq:1});
        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].qty, 2, "matched qty should equal the resting qty");
        // the remaining 3 units should now be resting on the bid side at price 100
        assert_eq!(b.best_bid(), Some(100), "unfilled remainder must rest on bid side");
        assert_eq!(b.best_ask(), None, "ask side must be empty after full consumption");
    }
}
