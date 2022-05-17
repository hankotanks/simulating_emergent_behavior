use std::rc::Rc;
use iced::canvas::{Cache, Cursor, Event, Geometry, Program};
use iced::{Canvas, Element, Rectangle};
use iced::canvas::Event::Mouse;
use iced::mouse::Event::ButtonPressed;
use iced_native::event::Status;
use crate::universe::{Cell, CellContents, Universe};

pub(crate) struct Simulation;

impl iced::Sandbox for Simulation {
    type Message = ();

    fn new() -> Self {
        Self
    }

    fn title(&self) -> String {
        String::from("Simulating Emergent Behavior")
    }

    fn update(&mut self, _message: Self::Message) {
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let ui = UniverseInterface::new();
        Canvas::new(ui)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    }
}

struct UniverseInterface {
    universe: Universe,
    cache: Cache
}

impl UniverseInterface {
    fn new() -> Self {
        Self {
            universe: Universe::new(100, 100, 100, 16, None),
            cache: Cache::default()
        }
    }
}

impl Program<()> for UniverseInterface {
    fn update(&mut self, event: Event, bounds: Rectangle, cursor: Cursor) -> (Status, Option<()>) {
        if let Mouse(event) = event {
            if let ButtonPressed(..) = event {
                println!("{}", self.point_to_cell(bounds, cursor));
            }
        }

        (Status::Ignored, None)
    }

    fn draw(&self, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry> {
        use iced::canvas::{Path, Fill};
        use iced::{Point, Color, Size};

        let cells = self.cache.draw(bounds.size(), |frame| {
            frame.fill(&Path::rectangle(Point::ORIGIN, frame.size()), Color::from_rgb8(0x40, 0x44, 0x4B));

            let size = (bounds.width / self.universe.width() as f32,
                        bounds.height / self.universe.height() as f32);

            for y in 0..self.universe.height() {
                for x in 0..self.universe.width() {
                    frame.fill_rectangle(Point::new(x as f32 * size.0, y as f32 * size.1), Size { width: size.0, height: size.1 }, Fill::from(
                        self.universe.get(x, y).color()
                    ));
                }
            }
        } );

        vec![cells]
    }
}

impl UniverseInterface {
    fn point_to_cell(&self, bounds: Rectangle, cursor: Cursor) -> &Cell {
        let x = (cursor.position().unwrap().x / (bounds.width / self.universe.width() as f32)) as usize;
        let y = (cursor.position().unwrap().y / (bounds.height / self.universe.height() as f32)) as usize;

        &self.universe.get(x, y)
    }
}