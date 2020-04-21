#![deny(clippy::all)]
mod application;
mod rendering;
mod simulation;
mod tools;
use application::{App, AppFlags};
use iced::{Application, Settings};

fn main() {
    App::run(Settings::with_flags(AppFlags::default()));
}
