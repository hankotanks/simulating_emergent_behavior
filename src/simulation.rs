use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::Formatter;

use iced::{Color, Element, Point, Rectangle, Size};
use iced::canvas::{Cache, Cursor, Event};

use iced_native::event::Status;

use crate::universe::{CellContents, Universe};

pub(crate) struct Simulation {
    universe: Rc<RefCell<Universe>>,
    description: String
}

impl iced::Sandbox for Simulation {
    type Message = Message;

    fn new() -> Self {
        Self {
            universe: {
                let size: Size<usize> = Size::new(32, 16);
                Rc::new(RefCell::new(Universe::new(size, 8, 64, None)))
            },
            description: String::from("")
        }
    }

    fn title(&self) -> String {
        String::from("Simulating Emergent Behavior")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::TooltipChanged(tt) => {
                self.description = tt;
            },
            Message::TooltipClear => self.description = String::from("")
        }

    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        use iced::Length;
        let ui = UniverseInterface::new(Rc::clone(&self.universe));
        let ui = iced::Canvas::new(ui)
            .width(Length::Fill)
            .height(Length::Fill);

        let tt: iced::Tooltip<Message> = iced::Tooltip::new(ui, self.description.as_str(), iced::tooltip::Position::FollowCursor);

        iced::Container::new(tt)
            .height(Length::Fill)
            .width(Length::Fill)
            .into()

    }
}

pub(crate) enum Message {
    TooltipChanged(String),
    TooltipClear
}

// TODO: This is messy
impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::TooltipChanged(contents) => write!(f, "{}", contents),
            Message::TooltipClear => write!(f, "")
        }
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
        // TODO: Remove this test and properly implement UniverseInterface::tick
        self.universe.as_ref().borrow_mut().cells[1][1].set_contents(CellContents::Food(10));
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
                    ButtonPressed(..) => None,
                    CursorMoved { position } => {
                        if let Some(cursor) = self.cursor {
                            let coords = self.cell_at(position);
                            Some(Message::TooltipChanged(self.universe.as_ref().borrow().cells[coords.1][coords.0].get_tooltip()))

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

            for y in 0..u.dimensions.height {
                for x in 0..u.dimensions.width {
                    frame.fill_rectangle(Point::new(x as f32 * size.0, y as f32 * size.1), Size { width: size.0, height: size.1 }, iced::canvas::Fill::from(
                        u.cells[y][x].color()
                    ));
                }
            }
        });

        vec![cells]
    }
}

// helper methods
impl UniverseInterface {
    fn cell_at(&self, point: Point) -> (usize, usize) {
        let bounds = self.bounds.unwrap();

        let u = self.universe.as_ref().borrow();

        let x = (point.x /
            (bounds.width / u.dimensions.width as f32)) as usize;
        let y = (point.y /
            (bounds.height / u.dimensions.height as f32)) as usize;

        (x, y)
    }

    fn cell_contents_at(&self, point: Point) -> Option<CellContents> {
        let coords = self.cell_at(point);
        self.universe.as_ref().borrow().cells[coords.1][coords.0].contents.clone()
    }
}