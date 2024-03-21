#![allow(dead_code)]
#![allow(unused_must_use)]

pub mod keygen;
pub mod sign;

mod common;
mod message;

mod protos {
    include!(concat!(env!("OUT_DIR"), "/protos.rs"));
}
