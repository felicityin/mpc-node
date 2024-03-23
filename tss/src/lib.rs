#![allow(dead_code)]
#![allow(unused_must_use)]

pub mod keygen;
pub mod sign;

mod message;
mod network;

mod protos {
    include!(concat!(env!("OUT_DIR"), "/protos.rs"));
}
