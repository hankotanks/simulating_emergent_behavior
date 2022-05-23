use std::rc::Rc;
use std::cell::RefCell;

use iced::{Color, Element, Point, Rectangle, Size};
use iced::canvas::{Cache, Cursor, Event};
use iced::widget::canvas::event::Status;

use crate::universe::{CellContents, Coordinate, Universe};

#[derive(Debug, Clone)]
pub(crate) enum Message {
    TooltipChanged(String),
    TooltipClear,
    DescriptionChanged(String),
    DescriptionClear
}

pub(crate) struct Simulation {
    universe: Rc<RefCell<Universe>>,
    description: String,
    description_state: iced::scrollable::State,
    tooltip: String
}

impl iced::Sandbox for Simulation {
    type Message = Message;

    fn new() -> Self {
        Self {
            universe: {
                let size: Size<usize> = Size::new(128, 128);
                Rc::new(RefCell::new(Universe::new(size, 256, 64, None)))
            },
            description: String::from(""),
            description_state: iced::scrollable::State::new(),
            tooltip: String::from("")
        }
    }

    fn title(&self) -> String {
        String::from("Simulating Emergent Behavior")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::TooltipChanged(tooltip_text) => self.tooltip = tooltip_text,
            Message::TooltipClear => self.tooltip = String::from(""),
            Message::DescriptionChanged(description_text) => self.description = description_text,
            Message::DescriptionClear => self.description = String::from("")
        }

    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        use iced::Length;
        let ui = UniverseInterface::new(Rc::clone(&self.universe));
        let ui = iced::Canvas::new(ui)
            .width(Length::FillPortion(2u16))
            .height(Length::Fill);

        let tt: iced::Tooltip<Message> = iced::Tooltip::new(ui, self.tooltip.as_str(), iced::tooltip::Position::FollowCursor);

        let desc = iced::Text::new(&*self.description)
            .width(Length::FillPortion(1u16))
            .height(Length::Fill);
        let desc = iced::Scrollable::new(&mut self.description_state)
            .push(desc)
            .width(Length::Fill)
            .height(Length::Fill);

        iced::Row::new()
            .push(tt)
            .push(desc)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

struct UniverseInterface {
    universe: Rc<RefCell<Universe>>,
    cache: Cache,
    bounds: Option<Rectangle>,
    cursor: Option<Cursor>,
    should_redraw: bool
}

impl UniverseInterface {
    fn new(universe: Rc<RefCell<Universe>>) -> Self {
        Self {
            universe,
            cache: Cache::default(),
            bounds: None,
            cursor: None,
            should_redraw: false
        }
    }

    fn tick(&mut self) {
        self.universe.as_ref().borrow_mut().update();
    }
}

impl iced::canvas::Program<Message> for UniverseInterface {
    fn update(&mut self, event: Event, bounds: Rectangle, cursor: Cursor) -> (Status, Option<Message>) {
        use iced::canvas::Event::*;

        if self.should_redraw {
            self.cache.clear();
            self.should_redraw = false;
        }

        self.bounds = Some(bounds);
        if let Some(pos) = cursor.position() {
            if bounds.contains(pos) {
                self.cursor = Some(cursor);
            } else {
                self.cursor = None;
            }
        } else {
            self.cursor = None;
        }

        return (Status::Ignored, match event {
            Mouse(event) => {
                use iced::mouse::Event::*;
                match event {
                    ButtonPressed(..) => {
                        if let Some(..) = self.cursor {
                            match self.contents_at(cursor.position().unwrap()) {
                                Some(contents) => {
                                    use CellContents::*;
                                    Some(match contents {
                                        Agent(agent) => Message::DescriptionChanged(format!("{}", agent)),
                                        _ => Message::DescriptionClear
                                    })
                                },
                                None => None
                            }
                        } else {
                            None
                        }
                    },
                    CursorMoved { position } => {
                        if let Some(..) = self.cursor {
                                Some(match self.contents_at(position) {
                                    Some(contents) => Message::TooltipChanged(format!("{}", contents)),
                                    None => Message::TooltipClear
                                })
                        } else {
                            Some(Message::TooltipClear)
                        }
                    },
                    _ => None
                }
            },
            Keyboard(event) => {
                use iced::keyboard::Event::*;
                if let KeyPressed { .. } = event {
                    self.tick();
                    self.should_redraw = true;
                    None
                } else {
                    None
                }
            }
        })
    }

    fn draw(&self, bounds: Rectangle, _cursor: Cursor) -> Vec<iced::canvas::Geometry> {
        let cells = self.cache.draw(bounds.size(), |frame| {
            frame.fill(&iced::canvas::Path::rectangle(Point::ORIGIN, frame.size()), Color::from_rgb8(0x40, 0x44, 0x4B));

            let u = self.universe.as_ref().borrow();
            let size = (bounds.width / u.dimensions.width as f32,
                        bounds.height / u.dimensions.height as f32);

            for (coord, cell) in u.cells().iter() {
                frame.fill_rectangle(Point::new(coord.x as f32 * size.0,  coord.y as f32 * size.1), Size { width: size.0, height: size.1 }, iced::canvas::Fill::from(
                    cell.color()
                ));
            }
        });

        vec![cells]
    }
}

// helper methods
impl UniverseInterface {
    fn contents_at(&self, point: Point) -> Option<CellContents> { // returns a copy of the cell's contents at a given point on the canvas
        let bounds = self.bounds.unwrap();

        let u = self.universe.as_ref().borrow();

        let coord = Coordinate::new(
            (point.x / (bounds.width / u.dimensions.width as f32)) as usize,
            (point.y / (bounds.height / u.dimensions.height as f32)) as usize
        );

        match u.get(&coord) {
            Some(cell) => {
                Some(cell.contents.clone())
            },
            None => None
        }
    }
}