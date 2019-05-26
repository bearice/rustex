use std::cmp::Ordering;
use std::str::FromStr;
extern crate bigdecimal;
extern crate num_bigint;
use num_bigint::*;
use bigdecimal::*;
pub type BigDec = BigDecimal;

pub fn floor_with_prec(x: &BigDec, prec: i64) -> BigDec {
    let a = BigDec::new(BigInt::from(10),-prec+1);
    let b = x*&a;
    let c = BigDec::from(b.to_bigint().unwrap());
    let d = &c/&a;
    //println!("floor=> a={:?} b={:?} c={:?} d={:?}",a,b,c,d);
    return d;
}

//(:order-id 98636 :action :sell-limit :price "10383.31316488943776" :amount "4590.912099969064" :unfilled-amount "4590.912099969064" :state :submitted)
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum OrderAction {
    SellLimit = 0b00,
    SellMarket = 0b01,
    BuyLimit = 0b10,
    BuyMarket = 0b11,
}
impl OrderAction {
    pub fn is_selling(&self) -> bool {
        match self {
            OrderAction::SellLimit => true,
            OrderAction::SellMarket => true,
            OrderAction::BuyLimit => false,
            OrderAction::BuyMarket => false,
        }
    }
    pub fn is_buying(&self) -> bool {
        !self.is_selling()
    }
    pub fn is_limited(&self) -> bool {
        match self {
            OrderAction::SellLimit => true,
            OrderAction::SellMarket => false,
            OrderAction::BuyLimit => true,
            OrderAction::BuyMarket => false,
        }
    }
    pub fn is_market(&self) -> bool {
        !self.is_limited()
    }
}
impl FromStr for OrderAction {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ":sell-limit" => Ok(OrderAction::SellLimit),
            ":buy-limit" => Ok(OrderAction::BuyLimit),
            ":sell-market" => Ok(OrderAction::SellMarket),
            ":buy-market" => Ok(OrderAction::BuyMarket),
            _ => Err(format!("unknown order-action: {}", s)),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OrderState {
    Canceled,
    Submitted,
    PartialFilled,
    PartialCanceled,
    Filled,
}
impl FromStr for OrderState {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ":canceled" => Ok(OrderState::Canceled),
            ":submitted" => Ok(OrderState::Submitted),
            ":partial-filled" => Ok(OrderState::PartialFilled),
            ":partial-canceled" => Ok(OrderState::PartialCanceled),
            ":filled" => Ok(OrderState::Filled),
            _ => Err(format!("unknown order-state: {}", s)),
        }
    }
}

type OrderId = u64;

#[derive(Debug, Clone)]
pub struct OrderRec {
    pub id: OrderId,
    pub action: OrderAction,
    pub price: BigDec,
    pub amount: BigDec,
    pub unfilled_amount: BigDec,
    pub state: OrderState,
}

impl OrderRec {
    pub fn projected_amount(&self) -> BigDec {
        let r = &self.unfilled_amount * &self.price;
        //r.with_prec(18*2);
        r
    }
}
impl FromStr for OrderRec {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_start_matches('(').trim_end_matches(')');
        let parts: Vec<&str> = s.split(" ").collect();
        if parts.len() != 12 {
            return Err(format!("malformd input: {}", s));
        }
        let parse_big_dec = |s: &str| {
            s.trim_start_matches('"')
                .trim_end_matches('"')
                .parse()
                //.map(|x:BigDec|x.with_prec(18))
                .map_err(|e| format!("{}", e))
        };
        let id = parts[1].parse().map_err(|e| format!("{}", e))?;
        let action = parts[3].parse()?;
        let price = parse_big_dec(parts[5])?;
        let amount = parse_big_dec(parts[7])?;
        let unfilled_amount = parse_big_dec(parts[9])?;
        let state = parts[11].parse()?;
        Ok(OrderRec {
            id,
            action,
            price,
            amount,
            unfilled_amount,
            state,
        })
    }
}

impl PartialEq for OrderRec {
    fn eq(&self, b: &Self) -> bool {
        self.id == b.id
    }
}
impl Eq for OrderRec {}

impl PartialOrd for OrderRec {
    fn partial_cmp(&self, b: &Self) -> Option<Ordering> {
        Some(self.cmp(b))
    }
}
impl Ord for OrderRec {
    fn cmp(&self, b: &Self) -> Ordering {
        assert_eq!(self.action,b.action);
        match self.price.cmp(&b.price) {
            //for buying orders, older(id smaller) is larger
            Ordering::Equal => if self.action.is_selling() {
                self.id.cmp(&b.id)
            }else{
                b.id.cmp(&self.id)
            },
            x => x,
        }
    }
}
//((:role :taker :order-id 1 :action :buy-market :price "0.0" :state :canceled :unfilled-amount "5086.1743187523466"))
/*(
    (:role :maker :order-id 10 :action :buy-limit :price "12545.2998023810023" :state :filled :filled-amount "822.4578316348483")
    (:role :taker :order-id 18 :action :sell-limit :price "12282.6631713243873" :state :partial-filled :unfilled-amount "4001.5522105302251"))
  )
  (
      (:role :maker :order-id 54 :action :sell-limit :price "10903.8400200751851" :state :filled :filled-amount "1232.206072806494684593")
      (:role :maker :order-id 46 :action :sell-limit :price "10948.9609178829178" :state :filled :filled-amount "2346.2435454556476")
      (:role :maker :order-id 61 :action :sell-limit :price "11629.2318214736783" :state :partial-filled :filled-amount "140.610619064097115407")
      (:role :taker :order-id 62 :action :buy-limit :price "13935.8485383696925" :state :filled :unfilled-amount "0.000000000000000000")
   )
*/
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MatchResultRole {
    Taker,
    Maker,
}

impl FromStr for MatchResultRole {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            ":maker" => Ok(MatchResultRole::Maker),
            ":taker" => Ok(MatchResultRole::Taker),
            _ => Err(format!("unknown order-state: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchResult {
    pub role: MatchResultRole,
    pub id: OrderId,
    pub action: OrderAction,
    pub price: BigDec,
    pub state: OrderState,
    pub amount: BigDec,
}

impl FromStr for MatchResult {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim_start_matches('(').trim_end_matches(')');
        let parts: Vec<&str> = s.split(" ").collect();
        if parts.len() != 12 {
            return Err(format!("malformd input: {}", s));
        }
        let parse_big_dec = |s: &str| {
            s.trim_start_matches('"')
                .trim_end_matches('"')
                .parse()
                //.map(|x:BigDec|x.with_prec(18))
                .map_err(|e| format!("{}", e))
        };
        let role = parts[1].parse()?;
        let id = parts[3].parse().map_err(|e| format!("{}", e))?;
        let action = parts[5].parse()?;
        let price = parse_big_dec(parts[7])?;
        let state = parts[9].parse()?;
        let amount = parse_big_dec(parts[11])?;
        Ok(MatchResult {
            role,
            id,
            action,
            price,
            amount,
            state,
        })
    }
}

impl MatchResult {
    pub fn from_line(s: String) -> Result<Vec<Self>, String> {
        let r = s.split(") (");
        Ok(r.map(|x| x.parse().unwrap()).collect())
    }
}

impl MatchResult {
    pub fn debug_vec_eq(x: &Vec<Self>, y: &Vec<Self>) -> bool{
        x.iter().zip(y.iter()).map(|(x,y)|{
            x.debug_eq(y)
        }).all(|x|x)
    }
    pub fn debug_eq(&self, x: &Self) -> bool {
        macro_rules! field_eq {
            ($e:ident) => {{
                let r = self.$e.eq(&x.$e);
                println!("{}->{}",stringify!($e),r);
                r
            }};
            ($e:ident, $($es:ident), +) => (
                field_eq!($e) && field_eq!($($es),+)
            );
        }
        field_eq!(id, role, action, price, amount, state)
    }
}
