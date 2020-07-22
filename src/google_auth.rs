use actix::prelude::*;
use actix::{Actor, Context};
use std::time::{Duration, SystemTime};

const MILLIS_BETWEEN: u64 = 400;

pub struct MyActor;

impl Actor for MyActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let mut last_interval_run = SystemTime::UNIX_EPOCH;
        ctx.run_interval(Duration::from_millis(MILLIS_BETWEEN), move |_, _| {
            let ts = SystemTime::now();
            println!(
                "I am alive! plus time: {:?}",
                ts.duration_since(last_interval_run).unwrap()
            );
            last_interval_run = ts;
        });
    }
}
