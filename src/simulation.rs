use iced::canvas::{Cache, Cursor, Event, Fill, Geometry, Path, Program};
use iced::{Application, Canvas, Color, Column, Container, Element, executor, Length, Rectangle, Size};
use iced_native::event::Status;
use iced_native::{Command, Point, Subscription};
use iced_native::mouse::Event::{ButtonPressed, CursorMoved};
use crate::universe::Cell;

#[derive(Debug)]
pub(crate) enum Message {
    EventOccurred(iced_native::Event),
    Toggled(bool),
    Exit
}

// TODO: the SimulationInterface should only reference Simulation methods, not Universe methods
struct Simulation {
    universe: crate::universe::Universe,
    cache: Cache,
    inspection_queued: bool,
    cursor: Point
}

impl Program<Message> for Simulation {
    fn update(&mut self, _event: Event, bounds: Rectangle, _cursor: Cursor) -> (Status, Option<Message>) {
        if self.inspection_queued {
            let x = (self.cursor.x / (bounds.width / self.universe.width() as f32)) as usize;
            let y = (self.cursor.y / (bounds.height / self.universe.height() as f32)) as usize;
            println!("{:?}", bounds);
            println!("{:?}", self.cursor);
            println!("{}, {}", x, y);
            self.inspection_queued = false;
        }

        (Status::Ignored, None)
    }

    fn draw(&self, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry> {
        let cells = self.cache.draw(bounds.size(), |frame| {
            frame.fill(&Path::rectangle(Point::ORIGIN, frame.size()), Color::from_rgb8(0x40, 0x44, 0x4B));

            let size = (bounds.width / self.universe.width() as f32,
                        bounds.height / self.universe.height() as f32);

            for y in 0..self.universe.height() {
                for x in 0..self.universe.width() {
                    frame.fill_rectangle(Point::new(x as f32 * size.0, y as f32 * size.1), Size { width: size.0, height: size.1 }, Fill::from(
                        match self.universe.get(x, y) {
                            Cell::Empty => Color::from_rgb8(0x40, 0x44, 0x4B),
                            Cell::Food(..) => Color::from_rgb8(0xFF, 0x64, 0x00),
                            Cell::Agent(..) => Color::from_rgb8(0x96, 0x64, 0xFF)
                        }
                    ));
                }
            }
        } );

        vec![cells]
    }
}

impl Simulation {
    pub fn view(&mut self) -> Element<Message> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

pub(crate) struct SimulationInterface {
    simulation: Simulation
}

impl Application for SimulationInterface {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (Self {
            simulation: Simulation {
                universe: crate::universe::Universe::new((100, 100), 100, 16, None),
                cache: Default::default(),
                inspection_queued: false,
                cursor: Point::default()
            }
        }, Command::none())
    }

    fn title(&self) -> String {
        "Simulating Emergent Behavior".parse().unwrap()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::EventOccurred(event) => {
                if let iced_native::Event::Mouse(mouse) = event {
                    match mouse {
                        ButtonPressed(..) => self.simulation.inspection_queued = true,
                        CursorMoved { position} => self.simulation.cursor = position,
                        _ => {}
                    }
                }
            },
            _ => {}
        }

        Command::none()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        iced_native::subscription::events().map(Message::EventOccurred)
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let content = Column::new()
            .push(
                self.simulation
                    .view()
            );

        Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill).into()
    }
}