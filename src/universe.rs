use std::fmt;
use std::fmt::Formatter;
use std::hash::Hash;
use std::cell::RefCell;
use std::collections::hash_map::Values;
use std::collections::HashMap;
use std::iter::Map;
use std::slice::Iter;

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

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub(crate) struct Coordinate {
    pub(crate) x: usize,
    pub(crate) y: usize
}

impl Coordinate {
    pub(crate) fn new(x: usize, y: usize) -> Self {
        Self { x, y }
    }

    pub(crate) fn offset(&mut self, offset: CoordinateOffset) {
        let mut x = self.x as isize + offset.x;
        let mut y = self.y as isize + offset.y;

        if x > 0 {
            x %= offset.dimensions.width as isize;
        } else if x < 0 {
            x = offset.dimensions.width as isize + x;
        }

        if y > 0 {
            y %= offset.dimensions.height as isize;
        } else if y < 0 {
            y = offset.dimensions.height as isize + y;
        }

        self.x = x as usize;
        self.y = y as usize;
    }
}

impl fmt::Display for Coordinate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

#[derive(Debug)]
pub(crate) struct CoordinateOffset<'a> {
    x: isize,
    y: isize,
    dimensions: &'a iced::Size<usize>
}

impl<'a> CoordinateOffset<'a> {
    pub(crate) fn new(x: isize, y: isize, dimensions: &'a iced::Size<usize>) -> Self {
        Self {
            x,
            y,
            dimensions
        }
    }

    fn from_facing(facing: crate::agent::Facing, dimensions: &'a iced::Size<usize>) -> Self {
        use crate::agent::Facing::*;

        let x: isize;
        let y: isize;

        match facing {
            Up => {
                x = 0;
                y = -1
            },
            Down => {
                x = 0;
                y = 1;
            },
            Left => {
                x = -1;
                y = 0;
            },
            Right => {
                x = 1;
                y = 0;
            }
        }

        return crate::universe::CoordinateOffset {
            x,
            y,
            dimensions
        }
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
    cells: HashMap<Coordinate, RefCell<Cell>>,
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
                let mut universe: HashMap<Coordinate, RefCell<Cell>> = HashMap::new();

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
                                        RefCell::new(c)
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

                universe
            },

            dimensions
        }
    }

    pub(crate) fn update(&mut self) {
        let mut births: Vec<Cell> = Vec::new();
        for (coord, cell) in self.cells.iter() {
            if let CellContents::Agent(ref mut agent) = cell.borrow_mut().contents {
                // check if the creature reproduces
                if thread_rng().gen_range(0..=255) < agent.fitness {
                    // get the birth coordinate and offset it appropriately
                    let mut birth_coord = coord.clone();
                    birth_coord.offset(CoordinateOffset::from_facing(agent.facing.opposite(), &self.dimensions));

                    // if there is an empty space behind it
                    if self.cells.get(&birth_coord).is_none() {
                        // reset the fitness of the parent, even if reproduction fails
                        agent.fitness = 0u8;

                        match crate::agent::Agent::from_string(agent.reproduce()) {
                            Ok(child) => {
                                let mut c = Cell::new(birth_coord);

                                // add the child to the new cell
                                c.contents = CellContents::Agent(child);

                                births.push(c);
                            },
                            Err(..) => {  } // do nothing if the offspring is non-viable
                        }
                    }
                }
            }
        }

        for cell in births.drain(0..births.len()) {
            self.cells.insert(cell.coord.clone(), RefCell::new(cell));
        }

        // perform action
        for cell in self.cells.values() {
            if let CellContents::Agent(agent) = &cell.borrow().contents {
                if let Some(action) = agent.resolve(&Sense::new(self, cell)) {
                    self.perform_action(cell, action);
                }
            }
        }
    }

    fn perform_action(&self, _cell: &RefCell<Cell>, _action: ActionType) {
        // TODO: Re-implement Universe::perform_action

    }
}

// helper methods
impl Universe {
    pub(crate) fn cells(&self) -> Vec<Cell> {
        self.cells.iter().map(|cell| {
            cell.1.borrow().clone()
        } ).collect::<Vec<Cell>>()
    }

    pub(crate) fn get(&self, coord: &Coordinate) -> Option<Cell> {
        match self.cells.get(coord) {
            Some(cell) => Some(cell.borrow().clone()),
            None => None
        }
    }
}

// TODO: Implement Sense struct
pub(crate) struct Sense {

}

impl Sense {
    pub(crate) fn new(_universe: &Universe, _cell: &RefCell<Cell>) -> Self {
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