mod gene;
mod agent;
mod universe;
mod simulation;

use iced::Application;
use iced::window;

pub fn main() -> iced::Result {
    simulation::Simulation::run(iced::Settings {
        antialiasing: true,
        window: window::Settings {
            position: window::Position::Centered,
            ..window::Settings::default()
        },
        ..iced::Settings::default()
    })
}

// TODO: Universe should wrap from opposite edges

// TODO: Pop up window to display genome and digraph.
//       Activated when clicking on an Agent