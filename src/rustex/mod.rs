extern crate bigdecimal;
extern crate skiplist;
use self::records::*;
use crate::rustex::bigdecimal::Zero;
use bigdecimal::*;
use skiplist::ordered_skiplist::OrderedSkipList;
use std::cell::RefCell;
pub mod records;

type Order = RefCell<OrderRec>;

#[derive(Debug)]
pub struct OrderBook {
    list: OrderedSkipList<Order>,
    price: Option<BigDec>,
}

// front < buy < sell < back
impl OrderBook {
    fn new() -> OrderBook {
        OrderBook {
            list: OrderedSkipList::with_capacity(65536),
            price: None,
        }
    }
}
pub struct Exchange {
    buyer: OrderBook,
    seller: OrderBook,
}

impl Exchange {
    pub fn new() -> Exchange {
        Exchange {
            buyer: OrderBook::new(),
            seller: OrderBook::new(),
        }
    }

    fn lookup_buyer(&mut self, o: &OrderRec) -> Option<Order> {
        //println!("lookup_buy {:?} > {}",&p,&o.price);
        let list = &mut self.buyer.list;
        if !list.is_empty()
            && (&list.back().unwrap().borrow().price >= &o.price || o.action.is_market())
        {
            self.buyer.list.pop_back()
        } else {
            None
        }
    }

    fn add_buyer(&mut self, o: Order) {
        self.buyer.list.insert(o);
    }

    fn lookup_seller(&mut self, o: &OrderRec) -> Option<Order> {
        //println!("lookup_sell {:?} < {}",&p,&o.price);
        let list = &mut self.seller.list;
        if !list.is_empty()
            && (&list.front().unwrap().borrow().price <= &o.price || o.action.is_market())
        {
            self.seller.list.pop_front()
        } else {
            None
        }
    }

    fn add_seller(&mut self, o: Order) {
        self.seller.list.insert(o);
    }

    fn add_order(&mut self, o: Order) {
        //if o.borrow().id==18 {println!("add_order => o: {:?}", &o);}
        if o.borrow().action.is_selling() {
            self.add_seller(o)
        } else {
            self.add_buyer(o)
        }
    }

    pub fn process(&mut self, o: Order) -> Box<Vec<MatchResult>> {
        let mut ret = Box::new(Vec::with_capacity(10));
        let mo = o.borrow();
        let id = mo.id;
        let action = mo.action;
        let price = mo.price.clone();
        let mut unfilled = mo.unfilled_amount.clone();
        drop(mo);
        loop {
            let m = if action.is_selling() {
                self.lookup_buyer(&o.borrow())
            } else {
                self.lookup_seller(&o.borrow())
            };

            //println!("lookup({:?}) => {:?}", o, m);
            if m.is_none() {
                break;
            }
            let m = m.unwrap();
            {
                let mm = m.borrow();
                let avail = if action == OrderAction::BuyMarket {
                    mm.projected_amount()
                } else {
                    mm.unfilled_amount.clone()
                };
                //unfilled = unfilled.with_prec(18);
                //if id==19 {println!("unfilled => {:?}, avail=> {:?}", &unfilled, &avail);}
                unfilled -= avail;
            }
            use std::cmp::Ordering;
            fn make_result(o: &OrderRec, state: OrderState, amount: &BigDec) -> MatchResult {
                MatchResult {
                    id: o.id,
                    role: MatchResultRole::Maker,
                    action: o.action,
                    price: o.price.clone(),
                    state,
                    amount: floor_with_prec(amount,18),
                }
            }

            let (res, done) = match unfilled.cmp(&BigDec::zero()) {
                //unfilled > 0 , comsume matched order, loop to next match
                Ordering::Greater => {
                    //state = OrderState::PartialFilled;
                    let mm = m.borrow();
                    let mut mo = o.borrow_mut();
                    let filled = if action == OrderAction::BuyMarket {
                        mm.projected_amount()
                    } else {
                        mm.unfilled_amount.clone()
                    };
                    mo.unfilled_amount -= &filled;
                    /*
                    println!(
                        "full-filled: {}, {}, {}, mo: {:?}",
                        mm.id, &mm.unfilled_amount, &filled, &o
                    );
                    */
                    let res = make_result(&mm, OrderState::Filled, &mm.unfilled_amount);
                    drop(mm);
                    drop(mo);
                    (res, false)
                }
                //unfilled < 0, stop and push match order back
                Ordering::Less => {
                    //state = OrderState::Filled;
                    let mut mm = m.borrow_mut();
                    let mo = o.borrow();
                    let filled = if action == OrderAction::BuyMarket {
                        floor_with_prec(&(&mo.unfilled_amount / &mm.price),18)
                    } else {
                        mo.unfilled_amount.clone()
                    };
                    /*
                    println!(
                        "partial-filled: {}, {}, {}, mo: {:?}",
                        mm.id, &mm.unfilled_amount, &filled, &o
                    );
                    */
                    mm.unfilled_amount -= &filled;
                    unfilled = if action == OrderAction::BuyMarket {
                        &mo.unfilled_amount - &filled * &mm.price
                    } else {
                        BigDec::zero()
                    };
                    let res = make_result(&mm, OrderState::PartialFilled, &filled);
                    drop(mm);
                    drop(mo);
                    self.add_order(m);
                    (res, true)
                }
                //unfilled = 0
                Ordering::Equal => {
                    //state = OrderState::Filled;
                    let mm = m.borrow();
                    let filled = &mm.unfilled_amount;
                    /*
                    println!(
                        "just-filled: {}, {}, {}, mo: {:?}",
                        mm.id, &mm.unfilled_amount, &filled, &o
                    );
                    */
                    let res = make_result(&mm, OrderState::Filled, filled);
                    drop(mm);
                    (res, true)
                }
            };
            //println!("res={:?}",res);
            ret.push(res);
            if done {
                break;
            }
        }
        //println!("{},{}",limited,unfilled);
        let limited = o.borrow().action.is_limited();
        let fully_filled = unfilled == BigDec::zero();
        let nothing_matched = ret.len() == 0;
        let state = match (limited, fully_filled, nothing_matched) {
            (_, true, _) => OrderState::Filled,
            (true, false, false) => OrderState::PartialFilled,
            (true, false, true) => OrderState::Submitted,
            (false, false, false) => OrderState::PartialCanceled,
            (false, false, true) => OrderState::Canceled,
            _ => panic!("not goona happens"),
        };
        //if current order not fully-filled
        if limited && unfilled != BigDec::zero() {
            self.add_order(o);
        }

        let res = MatchResult {
            id,
            role: MatchResultRole::Taker,
            action,
            price,
            state,
            amount: unfilled,
        };
        ret.push(res);
        ret
    }
}
