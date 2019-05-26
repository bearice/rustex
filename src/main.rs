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
    let mut ex = Exchange::new();
    let records: Vec<RefCell<OrderRec>> =
        load_dat("data/in.dat", |s| RefCell::new(s.unwrap().parse().unwrap()))?;
    let results: Vec<Vec<MatchResult>> = load_dat("data/out.dat", |s| {
        MatchResult::from_line(s.unwrap()).unwrap()
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
        /*
        if &res != y {
            println!("H {:?}", &res);
            println!("W {:?}", y);
            MatchResult::debug_vec_eq(&res,y);
            return Ok(());
        }
        */
    }
    let d = now.elapsed().as_millis();
    println!("Processed {} records in {} ms", &len, d);
    Ok(())
}
