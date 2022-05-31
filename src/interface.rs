use std::rc::Rc;
use std::cell::RefCell;
use std::fmt;

use iced::canvas;
use iced::canvas::event::Status;

use crate::tile;
use crate::tile::coord;
use crate::simulation::Simulation;

#[derive(Debug, Clone)]
pub(crate) enum Message {
    InspectorTarget(crate::agent::Agent),
    InspectorPaneChange(InspectorPane),
    InspectorCopy,
    Step,
}

pub(crate) struct Interface {
    simulation: Rc<RefCell<Simulation>>,
    target: Option<crate::agent::Agent>,
    selection: Option<InspectorPane>,
    selection_text: String,
    state_pick_list: iced::pick_list::State<InspectorPane>,
    state_copy: iced::button::State,
    state_scrollable: iced::scrollable::State
}

impl Default for Interface {
    fn default() -> Self {
        Self {
            simulation: Rc::new(RefCell::new(Simulation::default())),
            target: None,
            selection: Some(InspectorPane::default()),
            selection_text: String::default(),
            state_pick_list: iced::pick_list::State::default(),
            state_copy: iced::button::State::default(),
            state_scrollable: iced::scrollable::State::default()
        }
    }
}

impl iced::Sandbox for Interface {
    type Message = Message;

    fn new() -> Self {
        Self::default()
    }

    fn title(&self) -> String {
        String::from("Simulating Emergent Behavior")
    }

    fn update(&mut self, message: Self::Message) {
        use Message::*;
        match message {
            InspectorTarget(agent) => self.set_target(agent),
            InspectorPaneChange(pane) => self.set_selection(pane),
            InspectorCopy => arboard::Clipboard::new().unwrap().set_text(self.selection_text.clone()).unwrap(),
            Step => self.simulation.borrow_mut().step()
        }
    }

    fn view(&mut self) -> iced::Element<'_, Self::Message> {
        use iced::Length;

        let canvas = InterfaceCanvas::new(Rc::clone(&self.simulation)).view();

        // TODO: Move this into its own struct
        let inspector = self.inspector();

        iced::Row::new()
            .push(canvas)
            .push(inspector)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(iced::Padding::new(Self::PADDING))
            .spacing(Self::PADDING)
            .into()

    }
}

impl Interface {
    const PADDING: u16 = 10;

    fn inspector(&mut self) -> iced::Element<'_, Message> {
        use iced::Length;

        use Message::*;
        iced::Column::new()
            .push(
                iced::PickList::new(
                    &mut self.state_pick_list,
                    &InspectorPane::ALL[..],
                    self.selection,
                    InspectorPaneChange)
                    .width(Length::Fill))
            .push(
                iced::Scrollable::new(&mut self.state_scrollable)
                    .push(
                        iced::Text::new(&self.selection_text)
                            .width(Length::Fill)
                            .height(Length::Shrink))
                    .push(
                        iced::Button::new(
                            &mut self.state_copy,
                            iced::Text::new("Copy"))
                            .width(Length::Fill)
                            .on_press(InspectorCopy))
                    .width(Length::Fill)
                    .height(Length::Shrink)
                    .spacing(Self::PADDING))
            .width(Length::FillPortion(1u16))
            .height(Length::Shrink)
            .spacing(Self::PADDING)
            .into()
    }

    fn set_target(&mut self, agent: crate::agent::Agent) {
        self.target = Some(agent);

        self.update_selection_text();
    }

    fn set_selection(&mut self, pane: InspectorPane) {
        self.selection = Some(pane);

        self.update_selection_text();
    }

    fn update_selection_text(&mut self) {
        use InspectorPane::*;

        if self.target.is_none() {
            return;
        }

        // TODO: Messy! Clone should be avoided...
        let agent = self.target.clone().unwrap();
        self.selection_text = match self.selection.unwrap() {
            Genome => crate::agent::gene::Genome::get(agent.genome),
            Brain => format!("{}", petgraph::dot::Dot::new(&agent.brain)),
            History => {
                agent.history.iter().fold(String::new(), |output, action| {
                    output + &*format!("{:?}", action) + "\n"
                } )
                    .trim_end()
                    .to_string()
            }
        }
    }
}

struct InterfaceCanvas {
    simulation: Rc<RefCell<Simulation>>,
    cache: canvas::Cache,
    redraw: bool
}

impl InterfaceCanvas {
    const PADDING: u16 = 10;

    fn new(simulation: Rc<RefCell<Simulation>>) -> Self {
        Self {
            simulation,
            cache: canvas::Cache::new(),
            redraw: false
        }
    }

    fn view(self) -> iced::Element<'static, Message> {
        use iced::Length;
        iced::Canvas::new(self)
            .width(Length::FillPortion(2u16))
            .height(Length::Fill)
            .into()
    }
}

// Colors
impl InterfaceCanvas {
    const COLOR_WALL: [u8; 3] = [0x00, 0x00, 0x00];
    const COLOR_FOOD: [u8; 3] = [0xFF, 0x64, 0x64];
    const COLOR_AGENT: [u8; 3] = [0x64, 0x64, 0xFF];
    const COLOR_EMPTY: [u8; 3] = [0x1A, 0x1A, 0x1A];

    fn color(&self, tile: Option<&tile::Tile>) -> iced::Color {
        let to_color = |color: [u8; 3]| {
            [color[0] as f32 / 255f32, color[1] as f32 / 255f32, color[2] as f32 / 255f32]
        };

        if tile.is_none() {
            return iced::Color::from(to_color(Self::COLOR_EMPTY));
        }

        use tile::Tile::*;
        match tile.unwrap() {
            Agent(..) => iced::Color::from(to_color(Self::COLOR_AGENT)),
            Food(amount) => iced::Color::from_rgba8(
                Self::COLOR_FOOD[0],
                Self::COLOR_FOOD[1],
                Self::COLOR_FOOD[2],
                u8::from(*amount) as f32 / u8::from(ux::u3::MAX) as f32),
            Wall => iced::Color::from(to_color(Self::COLOR_WALL))
        }
    }
}

impl canvas::Program<Message> for InterfaceCanvas {
    fn update(&mut self, event: canvas::Event, bounds: iced::Rectangle, cursor: canvas::Cursor) -> (Status, Option<Message>) {
        // redraw if needed
        if self.redraw{
            self.cache.clear();

            self.redraw = false;
        }

        use canvas::event::Event::{Mouse, Keyboard};

        use iced::mouse::Event::*;
        use iced::keyboard::Event::*;

        use Message::*;

        let mut message: Option<Message> = None;
        match event {
            Mouse(ButtonPressed(..)) => {
                if let Some(coord) = self.coord_at(cursor, bounds) {
                    if self.simulation.borrow().exists(coord) {
                        let agent = self.simulation.borrow().get(coord).get_agent().clone();
                        message = Some(InspectorTarget(agent))
                    }
                }
            },
            Keyboard(KeyPressed { .. }) => {
                message = Some(Step);

                // the Canvas will be drawn next frame
                self.redraw = true;
            },
            _ => {  }
        }

        (Status::Ignored, message)
    }

    fn draw(&self, bounds: iced::Rectangle, _cursor: canvas::Cursor) -> Vec<canvas::Geometry> {
        let size = self.simulation.borrow().size();
        let size = (
            bounds.width / size.width as f32,
            bounds.height / size.height as f32
        );

        vec![
            self.cache.draw(bounds.size(), |frame| {
                frame.fill_rectangle(
                    iced::Point::new(0f32, 0f32),
                    bounds.size(),
                    self.color(None)
                );

                for coord in self.simulation.borrow().coords() {
                    let path = canvas::Path::circle(
                        iced::Point::new(
                            size.0 * (coord.x as f32 + 0.5f32),
                            size.1 * (coord.y as f32 + 0.5f32)
                        ),
                        (size.0 + size.1) / 4f32
                    );

                    frame.fill(
                        &path,
                        self.color(Some(self.simulation.borrow().get(coord)))
                    );
                }
            })
        ]
    }
}

// this block contains helper methods
impl InterfaceCanvas {
    // Returns None if there isn't a Tile at the given Point
    // Otherwise, returns the Coord of the Tile
    fn coord_at(&self, cursor: canvas::Cursor, bounds: iced::Rectangle) -> Option<coord::Coord> {
        // ensure the cursor is in the simulation window and above the Canvas
        cursor.position()?;
        if !bounds.contains(cursor.position().unwrap()) {
            return None;
        }


        let size = self.simulation.borrow().size();

        let point = cursor.position().unwrap();
        let coord = coord::Coord::new(
            ((point.x - Self::PADDING as f32) / (bounds.width / size.width as f32)) as usize,
            ((point.y - Self::PADDING as f32) / (bounds.height / size.height as f32)) as usize,
        );

        if self.simulation.borrow().exists(coord) {
            Some(coord)
        } else {
            None
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InspectorPane {
    Genome,
    Brain,
    History
}

impl InspectorPane {
    const ALL: [InspectorPane; 3] = [
        InspectorPane::Genome,
        InspectorPane::Brain,
        InspectorPane::History
    ];
}

impl Default for InspectorPane {
    fn default() -> Self {
        InspectorPane::Genome
    }
}

impl fmt::Display for InspectorPane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}",
               match self {
                   InspectorPane::Genome => "Genome",
                   InspectorPane::Brain => "Brain",
                   InspectorPane::History => "Action History"
               }
        )
    }
}