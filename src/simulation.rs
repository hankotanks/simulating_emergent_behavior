use std::rc::Rc;
use std::cell::RefCell;

use std::fmt;
use std::fmt::Formatter;

use iced::{Element, Point, Rectangle, Size};
use iced::canvas::{Cache, Cursor, Event};
use iced::widget::canvas::event::Status;

use crate::agent::Agent;
use crate::universe::{TileContents, Coordinate, Universe, Tile};

struct Color(u8, u8, u8);

impl Color {
    fn get(&self) -> iced::Color {
        iced::Color::from([self.0 as f32 / 255f32, self.1 as f32 / 255f32, self.2 as f32 / 255f32])
    }
}

const WALL_COLOR: Color = Color(0x00, 0x00, 0x00);
const FOOD_COLOR: Color = Color(0xFF, 0x64, 0x64);
const AGENT_COLOR: Color = Color(0x64, 0x64, 0xFF);
const EMPTY_COLOR: Color = Color(0x1A, 0x1A, 0x1A);

#[derive(Clone)]
pub(crate) enum Message {
    TooltipChanged(String),
    TooltipClear,
    DescriptionChanged(Agent),
    DescriptionPaneChanged(DescriptionPane),
    DescriptionClear,
    DescriptionCopy
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
        write!(f, "{}",
            match self {
                DescriptionPane::Genome => "Genome",
                DescriptionPane::Brain => "Brain"
            }
        )
    }
}

#[derive(Default)]
pub(crate) struct Simulation {
    universe: Rc<RefCell<Universe>>,
    state_pick_list: iced::pick_list::State<DescriptionPane>,
    state_scrollable: iced::scrollable::State,
    state_copy_button: iced::button::State,
    tooltip: String,
    description_target: Option<Agent>,
    description_text: String,
    selected_description_pane: Option<DescriptionPane>
}

impl iced::Sandbox for Simulation {
    type Message = Message;

    fn new() -> Self {
        // TODO: Disable copy button when no agent is selected
        Self {
            universe: Rc::new(RefCell::new(Universe::default())),
            state_scrollable: iced::scrollable::State::default(),
            state_pick_list: iced::pick_list::State::default(),
            state_copy_button: iced::button::State::default(),
            tooltip: String::default(),
            description_target: Option::default(),
            description_text: String::default(),
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
            },
            Message::DescriptionCopy => {
                arboard::Clipboard::new().unwrap().set_text(self.description_text.clone()).unwrap();
            }
        }

    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        use iced::Length;

        iced::Container::new(self.widgets())
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}

impl Simulation {
    fn update_tooltip(&mut self, tooltip: Option<String>) {
        self.tooltip = match tooltip {
            Some(text) => text,
            None => String::new()
        }
    }

    fn update_description(&mut self, agent: Option<Agent>) {
        // set the new description target
        self.description_target = agent;

        // update description text to match new agent
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

        // move to the top of the scrollable
        self.state_scrollable.snap_to(0f32);
    }

    fn widgets(&mut self) -> iced::Container<Message> {
        use iced::Length;

        // the dropdown where different DescriptionPanes can be selected
        let picker = iced::PickList::new(
            &mut self.state_pick_list,
            &DescriptionPane::ALL[..],
            self.selected_description_pane,
            Message::DescriptionPaneChanged)
            .width(Length::Fill);
        let picker = iced::Container::new(picker)
            .width(Length::Fill)
            .padding(iced::Padding::new(10));

        // a button used to copy the description
        let copy = iced::Button::new(
            &mut self.state_copy_button,
            iced::Text::new("Copy"))
            .width(Length::Fill)
            .on_press(Message::DescriptionCopy);
        let copy = iced::Container::new(copy)
            .width(Length::Fill)
            .padding(iced::Padding {
                top: 10,
                right: 0,
                bottom: 0,
                left: 0 } );

        // the area where the Agent's Genome and Digraph are displayed
        let desc = iced::Text::new(&self.description_text)
            .width(Length::FillPortion(1u16))
            .height(Length::Shrink);
        let desc = iced::Scrollable::new(&mut self.state_scrollable)
            .push(desc)
            .push(copy)
            .width(Length::Fill)
            .height(Length::Shrink)
            .scroller_width(0)
            .scrollbar_width(0)
            .padding(iced::Padding {
                top: 0,
                right: 10,
                bottom: 10,
                left: 10
            } );

        // put the inspection panel together
        let inspector = iced::Column::new()
            .push(picker)
            .push(desc)
            .width(Length::FillPortion(1u16))
            .height(Length::Fill);

        // start building the universe interface
        let ui = UniverseInterface::new(Rc::clone(&self.universe));
        let ui = iced::Canvas::new(ui)
            .width(Length::FillPortion(2u16))
            .height(Length::Fill);

        // add the tooltip element
        let ui: iced::Tooltip<Message> = iced::Tooltip::new(ui, self.tooltip.as_str(), iced::tooltip::Position::FollowCursor);

        // TODO: Add 10px border on Canvas element

        let content = iced::Row::new()
            .push(ui)
            .push(inspector)
            .width(Length::Fill)
            .height(Length::Fill);

        // wrap it in a container and return
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

        // get the message for this frame
        let mut message: Option<Message> = None;
        match event {
            Mouse(mouse_event) => {
                // ensure that only mouse events inside the canvas are processed
                if let Some(position) = cursor.position() {
                    if bounds.contains(position) {
                        message = self.process_mouse_event(mouse_event, cursor)
                    }
                }

            }, Keyboard(..) => {
                self.tick();
                self.should_redraw = true;
            }
        }

        (Status::Ignored, message)

    }

    fn draw(&self, bounds: Rectangle, _cursor: Cursor) -> Vec<iced::canvas::Geometry> {
        let tiles = self.cache.draw(bounds.size(), |frame| {
            // draw the background of the canvas
            frame.fill(
                &iced::canvas::Path::rectangle(Point::ORIGIN, frame.size()),
                EMPTY_COLOR.get());

            // maintain a mutable reference to the universe
            let u = self.universe.as_ref().borrow();

            // calculate the dimensions of each tile
            let size = (bounds.width / u.dimensions.width as f32,
                        bounds.height / u.dimensions.height as f32);

            // draw each tile
            for tile in u.tiles() {
                frame.fill_rectangle(
                    Point::new(tile.coord.x as f32 * size.0, tile.coord.y as f32 * size.1),
                    Size { width: size.0, height: size.1 },
                    iced::canvas::Fill::from(
                        match &tile.contents { // get the matching tile color
                            TileContents::Food(..) => FOOD_COLOR,
                            TileContents::Agent(..) => AGENT_COLOR,
                            TileContents::Wall => WALL_COLOR
                        }.get()
                    )
                );
            }
        });

        vec![tiles]
    }
}

// Helper methods
impl UniverseInterface {
    // Note that this returns a copy of the tile's contents
    fn contents_at(&self, point: Point) -> Option<TileContents> {
        let u = self.universe.as_ref().borrow();

        // get the coordinates of the tile
        let coord = Coordinate::new(
            (point.x / (self.bounds.unwrap().width / u.dimensions.width as f32)) as usize,
            (point.y / (self.bounds.unwrap().height / u.dimensions.height as f32)) as usize
        );

        let tile = u.get(&coord);
        match tile {
            Some(t) => {
                Some(t.contents.clone())
            }
            None => None
        }
    }

    fn process_mouse_event(&self, event: iced::mouse::Event, cursor: Cursor) -> Option<Message> {
        use iced::mouse::Event::*;

        // get the contents of the tile under the cursor
        let contents = if let Some(position) = cursor.position() {
            if let Some(contents) = self.contents_at(position) {
                Some(contents)
            } else {
                None
            }
        } else {
            None
        };

        let mut message: Option<Message> = None;
        match event {
            ButtonPressed(..) => {
                // change the inspection panel when a tile is clicked on
                message = Some(Message::DescriptionClear);
                if let Some(contents) = contents {
                    if let TileContents::Agent(agent) = contents {
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
            },
            _ => {}
        }


        message
    }
}