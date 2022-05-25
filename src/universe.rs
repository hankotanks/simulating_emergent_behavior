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
pub(crate) struct Tile {
    pub(crate) coord: Coordinate,
    pub(crate) contents: TileContents
}

impl fmt::Display for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cell @ {}: {}", self.coord, self.contents)
    }
}

impl Tile {
    pub(crate) fn new(coord: Coordinate) -> Self {
        Self {
            coord,
            contents: TileContents::Food(0)
        }
    }

    pub(crate) fn color(&self) -> iced::Color {
        match &self.contents {
            TileContents::Food(..) => FOOD_COLOR,
            TileContents::Agent(..) => AGENT_COLOR,
            TileContents::Wall => WALL_COLOR
        }.get()
    }
}

#[derive(Clone)]
pub(crate) enum TileContents {
    Food(u8),
    Agent(Agent),
    Wall
}

impl fmt::Display for TileContents {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            TileContents::Food(amount) => format!("Food ({})", amount),
            TileContents::Agent(agent) => format!("{}", agent),
            TileContents::Wall => String::from("Wall")
        })
    }
}

pub(crate) struct Universe {
    tiles: HashMap<Coordinate, RefCell<Tile>>,
    pub(crate) dimensions: iced::Size<usize>
}

impl Universe {
    pub(crate) fn new(dimensions: iced::Size<usize>, agents: usize, complexity: usize, seed: Option<u64>) -> Self {
        let mut prng: rand::rngs::StdRng = match seed {
            Some(s) => rand::SeedableRng::seed_from_u64(s),
            None => rand::SeedableRng::from_entropy()
        };

        Self {
            tiles: {
                let mut universe: HashMap<Coordinate, RefCell<Tile>> = HashMap::new();

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
                                        let mut c = Tile::new(coord);
                                        c.contents = TileContents::Agent(agent);
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
        let mut births: Vec<Tile> = Vec::new();
        for (coord, tile) in self.tiles.iter() {
            if let TileContents::Agent(ref mut agent) = tile.borrow_mut().contents {
                // check if the creature reproduces
                if thread_rng().gen_range(0..=255) < agent.fitness {
                    // get the birth coordinate and offset it appropriately
                    let mut birth_coord = coord.clone();
                    birth_coord.offset(CoordinateOffset::from_facing(agent.facing.opposite(), &self.dimensions));

                    // if there is an empty space behind it
                    if self.tiles.get(&birth_coord).is_none() {
                        // reset the fitness of the parent, even if reproduction fails
                        agent.fitness = 0u8;

                        match crate::agent::Agent::from_string(agent.reproduce()) {
                            Ok(child) => {
                                let mut t = Tile::new(birth_coord);

                                // add the child to the new tile
                                t.contents = TileContents::Agent(child);

                                births.push(t);
                            },
                            Err(..) => {  } // do nothing if the offspring is non-viable
                        }
                    }
                }
            }
        }

        for tile in births.drain(0..births.len()) {
            self.tiles.insert(tile.coord.clone(), RefCell::new(tile));
        }

        // perform action
        for tile in self.tiles.values() {
            if let TileContents::Agent(agent) = &tile.borrow().contents {
                if let Some(action) = agent.resolve(&Sense::new(self, tile)) {
                    self.perform_action(tile, action);
                }
            }
        }
    }

    fn perform_action(&self, _tile: &RefCell<Tile>, _action: ActionType) {
        // TODO: Re-implement Universe::perform_action

    }
}

// helper methods
impl Universe {
    pub(crate) fn tiles(&self) -> Vec<Tile> {
        self.tiles.iter().map(|tile| {
            tile.1.borrow().clone()
        } ).collect::<Vec<Tile>>()
    }

    pub(crate) fn get(&self, coord: &Coordinate) -> Option<Tile> {
        match self.tiles.get(coord) {
            Some(tile) => Some(tile.borrow().clone()),
            None => None
        }
    }
}

// TODO: Implement Sense struct
pub(crate) struct Sense {

}

impl Sense {
    pub(crate) fn new(_universe: &Universe, _tile: &RefCell<Tile>) -> Self {
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