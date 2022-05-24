use std::fmt;
use std::fmt::Formatter;
use std::hash::Hash;
use std::cell::RefCell;
use std::collections::HashMap;

use rand::{Rng, thread_rng};

use crate::agent::Agent;
use crate::gene::{ActionType, SenseType};

struct Color(u8, u8, u8);

impl Color {
    fn get(&self) -> iced::Color {
        iced::Color::from([self.0 as f32 / 255f32, self.1 as f32 / 255f32, self.2 as f32 / 255f32])
    }
}

const AGENT_COLOR: Color = Color(0x96, 0x64, 0xFF);
const WALL_COLOR: Color = Color(0xFF, 0xFF, 0xFF);
const FOOD_COLOR: Color = Color(0xFF, 0x64, 0x00);

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) struct Coordinate {
    pub(crate) x: usize,
    pub(crate) y: usize
}

impl Coordinate {
    pub(crate) fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[derive(Clone)]
pub(crate) struct Cell {
    pub(crate) coord: Coordinate,
    pub(crate) contents: CellContents
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cell @ {}: {}", self.coord, self.contents)
    }
}

impl Cell {
    pub(crate) fn new(coord: Coordinate) -> Self {
        Self {
            coord,
            contents: CellContents::Food(0)
        }
    }

    pub(crate) fn color(&self) -> iced::Color {
        match &self.contents {
            CellContents::Food(..) => FOOD_COLOR,
            CellContents::Agent(..) => AGENT_COLOR,
            CellContents::Wall => WALL_COLOR
        }.get()
    }
}

#[derive(Clone)]
pub(crate) enum CellContents {
    Food(u8),
    Agent(Agent),
    Wall
}

impl fmt::Display for CellContents {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            CellContents::Food(amount) => format!("Food ({})", amount),
            CellContents::Agent(agent) => format!("{}", agent),
            CellContents::Wall => String::from("Wall")
        })
    }
}

pub(crate) struct Universe {
    cells: RefCell<HashMap<Coordinate, Cell>>,
    pub(crate) dimensions: iced::Size<usize>
}

impl Universe {
    pub(crate) fn new(dimensions: iced::Size<usize>, agents: usize, complexity: usize, seed: Option<u64>) -> Self {
        let mut prng: rand::rngs::StdRng = match seed {
            Some(s) => rand::SeedableRng::seed_from_u64(s),
            None => rand::SeedableRng::from_entropy()
        };

        Self {
            cells: {
                let mut universe: HashMap<Coordinate, Cell> = HashMap::new();

                for _ in 0..agents {
                    'occupied: loop {
                        let coord = Coordinate::new(
                            prng.gen_range(0..dimensions.width),
                            prng.gen_range(0..dimensions.height)
                        );

                        if !universe.contains_key(&coord) {
                            match Agent::from_seed(complexity, &mut prng) {
                                Ok(agent) => {
                                    universe.insert(coord, {
                                        let mut c = Cell::new(coord);
                                        c.contents = CellContents::Agent(agent);
                                        c
                                    });
                                    break 'occupied;
                                },
                                Err(..) => {
                                    continue 'occupied;
                                }
                            }
                        }
                    }
                }

                RefCell::new(universe)
            },

            dimensions
        }
    }

    pub(crate) fn update(&mut self) {
        for (coord, cell) in self.cells.borrow_mut().iter_mut() {
            if let CellContents::Agent(ref mut agent) = cell.contents {
                // check if the creature reproduces
                if thread_rng().gen_range(0..255) < agent.fitness {
                    if let Some(birth_coord) = agent.facing.opposite().transform(coord, self.dimensions) {
                        // if there is an empty space behind it
                        if self.cells.borrow().get(&birth_coord).is_none() {
                            // reset the fitness of the parent, even if reproduction fails
                            agent.fitness = 0u8;

                            match crate::agent::Agent::from_string(agent.reproduce()) {
                                Ok(child) => {
                                    let mut c = Cell::new(birth_coord);

                                    // add the child to the new cell
                                    c.contents = CellContents::Agent(child);

                                    self.cells.borrow_mut().insert(birth_coord, c);
                                },
                                Err(..) => {  } // do nothing if the offspring is non-viable
                            }
                        }
                    }
                }
            }
        }

        // perform action
        for cell in self.cells.borrow().values() {
            if let CellContents::Agent(agent) = &cell.contents {
                if let Some(action) = agent.resolve(&Sense::new(self, cell)) {
                    self.perform_action(cell, action);
                }
            }
        }
    }

    fn perform_action(&self, _cell: &Cell, _action: ActionType) {
        // TODO: Re-implement Universe::perform_action

    }
}

// helper methods
impl Universe {
    pub(crate) fn cells(&self) -> std::cell::Ref<'_, HashMap<Coordinate, Cell>> {
        self.cells.borrow()
    }

    pub(crate) fn get(&self, coord: &Coordinate) -> Option<Cell> {
        match self.cells.borrow().get(coord) {
            Some(cell) => Some(cell.clone()),
            None => None
        }
    }
}

// TODO: Implement Sense struct
pub(crate) struct Sense {

}

impl Sense {
    pub(crate) fn new(_universe: &Universe, _cell: &Cell) -> Self {
        Self {

        }
    }

    pub(crate) fn get(&self, sense: &SenseType) -> f32 {
        // use crate::gene::SenseType::*;
        match sense {
            _ => 1f32
        }
    }
}

// TODO: Universe has wrapping edges