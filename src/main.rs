mod agent;
mod universe;
mod simulation;
mod tile;

use iced::Application;
use iced::window;

pub fn main() -> iced::Result {
    tile::test();
    /*
    simulation::Simulation::run(iced::Settings {
        antialiasing: true,
        window: window::Settings {
            position: window::Position::Centered,
            ..window::Settings::default()
        },
        ..iced::Settings::default()
    })

     */

    iced::Result::Ok(())
}