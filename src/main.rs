mod agent;
mod tile;
mod simulation;
mod interface;

use iced::Sandbox;

pub fn main() -> iced::Result {
    interface::Interface::run(iced::Settings::default())

}