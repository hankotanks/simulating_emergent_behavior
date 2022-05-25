use std::fmt;
use std::fmt::Formatter;
use std::hash::Hash;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::drop_in_place;

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
    pub(crate) fn new(coord: Coordinate, contents: TileContents) -> Self {
        Self {
            coord,
            contents
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

impl Default for Universe {
    fn default() -> Self {
        let dimensions: iced::Size<usize> = iced::Size::new(128, 128);
        Self::new(dimensions, 128, 64, None)
    }
}

impl Universe {
    pub(crate) fn new(dimensions: iced::Size<usize>, agents: usize, complexity: usize, seed: Option<u64>) -> Self {
        let mut prng: rand::rngs::StdRng = match seed {
            Some(s) => rand::SeedableRng::seed_from_u64(s),
            None => rand::SeedableRng::from_entropy()
        };

        let mut u = Self {
            tiles: HashMap::new(),
            dimensions
        };

        for _ in 0..agents {
            'occupied: loop {
                let coord = Coordinate::new(
                    prng.gen_range(0..dimensions.width),
                    prng.gen_range(0..dimensions.height)
                );

                if u.get(&coord).is_none() {
                    match Agent::from_prng(complexity, &mut prng) {
                        Ok(agent) => {
                            u.put(Tile::new(coord, TileContents::Agent(agent)));
                            break 'occupied;
                        },
                        Err(..) => {
                            continue 'occupied;
                        }
                    }
                }
            }
        }

        u
    }

    pub(crate) fn update(&mut self) {
        let mut births: Vec<Tile> = Vec::new();
        for coord in self.coords() {
            if let TileContents::Agent(ref mut agent) = self.get_mut(&coord).unwrap().contents {
                if thread_rng().gen_range(0..=255) < agent.fitness {
                    let mut child_coord = coord.clone();
                    let child_offset = CoordinateOffset::from_facing(agent.facing.opposite(), &self.dimensions);
                    child_coord.offset(child_offset);

                    if self.get(&child_coord).is_none() {
                        agent.fitness = 0u8;

                        if let Ok(child) = agent.reproduce() {
                            births.push(Tile::new(child_coord, TileContents::Agent(child)));
                        }
                    }
                }
            }
        }

        // add new births to the HashMap of tiles
        for tile in births.drain(0..births.len()) {
            self.put(tile);
        }

        // perform action
        for tile in self.tiles.values() {
            if let TileContents::Agent(agent) = &tile.borrow().contents {
                if let Some(action) = agent.resolve(&Sense::new(self, tile)) {
                    self.perform_action(tile.borrow().coord, action);
                }
            }
        }
    }

    fn perform_action(&self, _coord: Coordinate, _action: ActionType) {
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

    pub(crate) fn coords(&self) -> Vec<Coordinate> {
        self.tiles.keys().cloned().collect::<Vec<Coordinate>>()
    }

    pub(crate) fn get(&self, coord: &Coordinate) -> Option<std::cell::Ref<'_, Tile>> {
        match self.tiles.get(coord) {
            Some(tile) => Some(tile.borrow()),
            None => None
        }
    }

    fn get_mut(&self, coord: &Coordinate) -> Option<std::cell::RefMut<'_, Tile>> {
        match self.tiles.get(coord) {
            Some(tile) => Some(tile.borrow_mut()),
            None => None
        }
    }

    fn put(&mut self, tile: Tile) -> Option<Tile> {
        match self.tiles.insert(tile.coord, RefCell::new(tile)) {
            Some(old) => Some(old.borrow().clone()),
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