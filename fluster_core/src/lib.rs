#![deny(clippy::all)]
#![feature(div_duration)]
#![feature(clamp)]

#[macro_use]
extern crate nom;

pub mod actions;
pub mod ecs;
pub mod engine;
mod quad_tree;
pub mod rendering;
pub mod runner;
pub mod serialization;
pub mod tween;
pub mod types;
mod util;
