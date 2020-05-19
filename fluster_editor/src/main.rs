#![deny(clippy::all)]
mod application;
mod messages;
mod rendering;
mod simulation;
mod tools;
use application::{App, AppFlags};
use iced::{Application, Settings};

fn main() {
    App::run(Settings::with_flags(AppFlags::default()));
}
