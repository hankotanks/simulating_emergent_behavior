use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;
use std::fmt::Formatter;

use iced::{Color, Element, Point, Rectangle, Size};
use iced::canvas::{Cache, Cursor, Event};
use iced::widget::canvas::event::Status;
use crate::agent::Agent;

use crate::universe::{CellContents, Coordinate, Universe};

#[derive(Clone)]
pub(crate) enum Message {
    TooltipChanged(String),
    TooltipClear,
    DescriptionChanged(Agent),
    DescriptionPaneChanged(DescriptionPane),
    DescriptionClear
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use Message::*;
        write!(f, "{}", {
            match self {
                DescriptionChanged(agent) => format!("Description changed to describe an {}", agent),
                _ => format!("{:?}", self)
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DescriptionPane {
    Genome,
    Brain
}

impl DescriptionPane {
    const ALL: [DescriptionPane; 2] = [
        DescriptionPane::Genome,
        DescriptionPane::Brain
    ];
}

impl Default for DescriptionPane {
    fn default() -> Self {
        DescriptionPane::Genome
    }
}

impl fmt::Display for DescriptionPane {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub(crate) struct Simulation {
    universe: Rc<RefCell<Universe>>,
    state_pick_list: iced::pick_list::State<DescriptionPane>,
    state_scrollable: iced::scrollable::State,
    tooltip: String,
    description_target: Option<Agent>,
    description_text: String,
    selected_description_pane: Option<DescriptionPane>
}

impl iced::Sandbox for Simulation {
    type Message = Message;

    fn new() -> Self {
        // TODO: Implement Default for Simulation
        Self {
            universe: {
                let size: Size<usize> = Size::new(128, 128);
                Rc::new(RefCell::new(Universe::new(size, 256, 64, None)))
            },
            state_scrollable: iced::scrollable::State::new(),
            state_pick_list: iced::pick_list::State::new(),
            tooltip: String::from(""),
            description_target: None,
            description_text: String::from(""),
            selected_description_pane: Some(DescriptionPane::default()),
        }
    }

    fn title(&self) -> String {
        String::from("Simulating Emergent Behavior")
    }

    fn update(&mut self, message: Self::Message) {
        match message {
            Message::TooltipChanged(tooltip) => self.update_tooltip(Some(tooltip)),
            Message::TooltipClear => self.update_tooltip(None),
            Message::DescriptionChanged(agent) => self.update_description(Some(agent)),
            Message::DescriptionClear => self.update_description(None),
            Message::DescriptionPaneChanged(pane) => {
                self.selected_description_pane = Some(pane);

                self.update_description(self.description_target.clone());
            }
        }

    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        use iced::Length;
        let ui = UniverseInterface::new(Rc::clone(&self.universe));
        let ui = iced::Canvas::new(ui)
            .width(Length::FillPortion(2u16))
            .height(Length::Fill);

        let tt: iced::Tooltip<Message> = iced::Tooltip::new(ui, self.tooltip.as_str(), iced::tooltip::Position::FollowCursor);

        iced::Row::new()
            .push(tt)
            .push(self.inspect())
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

impl Simulation {
    fn update_tooltip(&mut self, tooltip: Option<String>) {
        self.tooltip = match tooltip {
            Some(text) => text,
            None => String::from("")
        }
    }

    fn update_description(&mut self, agent: Option<Agent>) {
        self.description_target = agent;
        match &self.description_target {
            Some(agent) => {
                if let Some(pane) = self.selected_description_pane {
                    self.description_text = match pane {
                        DescriptionPane::Genome => agent.get_genome_string(),
                        DescriptionPane::Brain => agent.get_digraph()
                    }
                }
            },
            None => self.description_text.clear()
        }

        self.state_scrollable.snap_to(0f32);
    }

    fn inspect(&mut self) -> iced::Container<Message> {
        use iced::Length;

        let picker = iced::PickList::new(&mut self.state_pick_list, &DescriptionPane::ALL[..], self.selected_description_pane, Message::DescriptionPaneChanged)
            .width(Length::Fill);
        let picker = iced::Container::new(picker)
            .width(Length::Fill)
            .padding(iced::Padding::new(10));

        let desc = iced::Text::new(&self.description_text)
            .width(Length::FillPortion(1u16))
            .height(Length::Fill);
        let desc = iced::Scrollable::new(&mut self.state_scrollable)
            .push(desc)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding::new(10));

        let content = iced::Column::new()
            .push(picker)
            .push(desc);

        iced::Container::new(content)
            .width(Length::Fill)
            .height(Length::Fill)
    }
}

struct UniverseInterface {
    universe: Rc<RefCell<Universe>>,
    cache: Cache,
    bounds: Option<Rectangle>,
    should_redraw: bool
}

impl UniverseInterface {
    fn new(universe: Rc<RefCell<Universe>>) -> Self {
        Self {
            universe,
            cache: Cache::default(),
            bounds: None,
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

        // redraw the scene if needed
        if self.should_redraw {
            self.cache.clear();
            self.should_redraw = false;
        }

        // update the bounds (this field is used by helper functions)
        self.bounds = Some(bounds);

        // get the contents of the cell under the cursor
        let contents = if let Some(position) = cursor.position() {
            if let Some(contents) = self.contents_at(position) {
                Some(contents)
            } else {
                None
            }
        } else {
            None
        };

        // get the message for this frame
        let mut message: Option<Message> = None;
        match event {
            Mouse(mouse_event) => {
                use iced::mouse::Event::*;
                match mouse_event {
                    ButtonPressed(..) => {
                        // change the inspection panel when a cell is clicked on
                        message = Some(Message::DescriptionClear);
                        if let Some(contents) = contents {
                            if let CellContents::Agent(agent) = contents {
                                message = Some(Message::DescriptionChanged(agent));
                            }
                        }
                    },
                    CursorMoved { .. } => {
                        // update tooltip when hovering over non-empty tiles
                        message = Some(Message::TooltipClear);
                        if let Some(contents) = contents {
                            message = Some(Message::TooltipChanged(format!("{}", contents)));
                        }
                    }, _ => {  }
                }
            }, _ => {  }
        }

        (Status::Ignored, message)

    }

    fn draw(&self, bounds: Rectangle, _cursor: Cursor) -> Vec<iced::canvas::Geometry> {
        let cells = self.cache.draw(bounds.size(), |frame| {
            // draw the background of the canvas
            frame.fill(
                &iced::canvas::Path::rectangle(Point::ORIGIN, frame.size()),
                Color::from_rgb8(0x40, 0x44, 0x4B));

            // maintain a mutable reference to the universe
            let u = self.universe.as_ref().borrow();

            // calculate the dimensions of each cell
            let size = (bounds.width / u.dimensions.width as f32,
                        bounds.height / u.dimensions.height as f32);

            // draw each cell
            for (coord, cell) in u.cells().iter() {
                frame.fill_rectangle(Point::new(coord.x as f32 * size.0,  coord.y as f32 * size.1), Size { width: size.0, height: size.1 }, iced::canvas::Fill::from(
                    cell.color()
                ));
            }
        });

        vec![cells]
    }
}

// Helper methods
impl UniverseInterface {
    // Note that this returns a copy of the cell's contents
    fn contents_at(&self, point: Point) -> Option<CellContents> {
        let u = self.universe.as_ref().borrow();

        // get the coordinates of the cell
        let coord = Coordinate::new(
            (point.x / (self.bounds.unwrap().width / u.dimensions.width as f32)) as usize,
            (point.y / (self.bounds.unwrap().height / u.dimensions.height as f32)) as usize
        );

        match u.get(&coord) {
            Some(cell) => {
                Some(cell.contents.clone())
            },
            None => None
        }
    }
}