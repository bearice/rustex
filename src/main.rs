use std::convert::TryFrom;
use rustex::records::*;
use rustex::Exchange;
use std::cell::RefCell;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
extern crate atomic_counter;

use std::time::Instant;
mod rustex;

fn load_dat<T, F>(name: &str, func: F) -> io::Result<Vec<T>>
where
    F: FnMut(io::Result<String>) -> T,
{
    let f = File::open(name)?;
    let reader = BufReader::new(f);
    let now = Instant::now();
    let records: Vec<T> = reader.lines().map(func).collect();
    println!(
        "Loaded {} records in {} ms",
        records.len(),
        now.elapsed().as_millis()
    );
    Ok(records)
}

fn main() -> io::Result<()> {
    /*
    let a: BigDec = "12345678901234567890.123456789012345678901234567890".parse().unwrap();
    println!("{:?}",a);
    let b = a.with_scale(8);
    println!("{:?}",b);
    let c = a.with_prec(8);
    println!("{:?}",c);
    return Ok(());
    */

    let mut ex = Exchange::new();
    let records: Vec<RefCell<OrderRec>> =
        load_dat("data/in.dat", |s| RefCell::new(s.unwrap().parse().unwrap()))?;
    let results: Vec<Box<Vec<MatchResult>>> = load_dat("data/out.dat", |s| {
        Box::new(MatchResult::from_line(s.unwrap()).unwrap())
    })?;
    let len = records.len();
    assert_eq!(len, results.len());
    let it = records.into_iter();
    let mut my_res = Vec::<Box<Vec<MatchResult>>>::with_capacity(len);
    let now = Instant::now();
    for x in it {
        //println!("\r\n===\r\nevent => {:?}", x);
        let res = ex.process(x);
        my_res.push(res);
    }
    let d = now.elapsed().as_millis();
    let speed = len * 1000 / usize::try_from(d).unwrap();
    println!("Processed {} records in {} ms, speed {}/s", &len, d, speed);
    let it = my_res.iter();
    let it2 = results.iter();
    let now = Instant::now();
    for (x,y) in it.zip(it2){
        if x != y {
            println!("H {:?}", x);
            println!("W {:?}", y);
            MatchResult::debug_vec_eq(x,y);
            return Ok(());
        }
    }
    let d = now.elapsed().as_millis();
    println!("Checked {} records in {} ms", &len, d);
    Ok(())
}
