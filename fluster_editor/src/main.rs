#![deny(clippy::all)]
mod application;
mod rendering;
mod simulation;
use application::App;
use iced::{Application, Settings};

fn main() {
    App::run(Settings::default());
}
