#![deny(clippy::all)]
mod application;
mod rendering;
mod simulation;
mod tools;
use application::{App, AppFlags};
use iced::{Application, Settings};
use pathfinder_geometry::vector::Vector2I;

fn main() {
    App::run(Settings::with_flags(AppFlags::new(Vector2I::new(800, 600))));
}
