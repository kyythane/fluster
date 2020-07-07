#![deny(clippy::all)]
#![feature(div_duration)]
#![feature(clamp)]
// #![feature(generators, generator_trait)]

#[macro_use]
extern crate nom;

pub mod actions;
pub mod ecs;
pub mod rendering;
pub mod runner;
pub mod serialization;
pub mod tween;
pub mod types;
pub mod util;
mod quad_tree;
