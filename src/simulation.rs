use std::rc::Rc;
use std::cell::RefCell;

use iced::canvas::{Cache, Cursor, Event, Fill, Geometry, Path, Program};
use iced::{Color, Element, Length, Point, Rectangle, Size};
use iced::canvas::Event::{Keyboard, Mouse};
use iced::keyboard::Event::KeyPressed;
use iced::mouse::Event::ButtonPressed;
use iced_native::event::Status;

use crate::universe::{CellContents, Universe};

pub(crate) struct Simulation {
    universe: Rc<RefCell<Universe>>
}

impl iced::Sandbox for Simulation {
    type Message = ();

    fn new() -> Self {
        Self {
            universe: {
                let size: Size<usize> = Size::new(32, 16);
                Rc::new(RefCell::new(Universe::new(size, 8, 64, None)))
            }
        }
    }

    fn title(&self) -> String {
        String::from("Simulating Emergent Behavior")
    }

    fn update(&mut self, _message: Self::Message) {

    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let ui = UniverseInterface::new(Rc::clone(&self.universe));
        iced::Canvas::new(ui)
            .width(Length::Fill)
            .height(Length::Fill)
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
        // TODO: Remove this test and properly implement UniverseInterface::tick
        self.universe.as_ref().borrow_mut().cells[1][1].set_contents(CellContents::Food(10));
    }
}

impl Program<()> for UniverseInterface {
    fn update(&mut self, event: Event, bounds: Rectangle, cursor: Cursor) -> (Status, Option<()>) {
        if self.should_redraw {
            self.cache.clear();
            self.should_redraw = false;
        }

        self.bounds = Some(bounds);
        self.cursor = Some(cursor);

        match event {
            Mouse(event) => {
                if let ButtonPressed(..) = event {
                    // TODO: Inspection should be a sub-window
                    let coords = self.cell_at();
                    println!("{}", self.universe.as_ref().borrow().cells[coords.1][coords.0]);
                }
            },
            Keyboard(event) => {
                if let KeyPressed { .. } = event {
                    self.tick();
                    self.should_redraw = true;
                }
            }
        }

        (Status::Ignored, None)
    }

    fn draw(&self, bounds: Rectangle, _cursor: Cursor) -> Vec<Geometry> {
        let cells = self.cache.draw(bounds.size(), |frame| {
            frame.fill(&Path::rectangle(Point::ORIGIN, frame.size()), Color::from_rgb8(0x40, 0x44, 0x4B));

            let u = self.universe.as_ref().borrow();

            let size = (bounds.width / u.dimensions.width as f32,
                        bounds.height / u.dimensions.height as f32);

            for y in 0..u.dimensions.height {
                for x in 0..u.dimensions.width {
                    frame.fill_rectangle(Point::new(x as f32 * size.0, y as f32 * size.1), Size { width: size.0, height: size.1 }, Fill::from(
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
    fn cell_at(&self) -> (usize, usize) {
        let bounds = self.bounds.unwrap();
        let cursor = self.cursor.unwrap();

        let u = self.universe.as_ref().borrow();

        let x = (cursor.position().unwrap().x /
            (bounds.width / u.dimensions.width as f32)) as usize;
        let y = (cursor.position().unwrap().y /
            (bounds.height / u.dimensions.height as f32)) as usize;

        (x, y)
    }
}