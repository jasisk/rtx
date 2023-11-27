#![allow(unused_imports)]
#[macro_use]
extern crate log;
#[macro_use]
extern crate rtx_common;

pub mod products;

pub use products::node;
pub use products::node::Node;
pub use products::Product;

pub(crate) mod built_info {
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}
