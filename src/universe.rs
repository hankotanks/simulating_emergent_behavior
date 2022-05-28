use std::borrow::BorrowMut;
use std::fmt;
use std::fmt::Formatter;
use std::hash::Hash;
use std::cell::RefCell;
use std::collections::HashMap;

use rand::{Rng, thread_rng};

use crate::r#mod::{Agent, Facing};
use crate::gene::{ActionType, SenseType};

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

    pub(crate) fn neighbors(&self, dimensions: &iced::Size<usize>) -> Vec<Coordinate> {
        use Facing::*;

        let mut neighbors = Vec::new();
        for face in vec![Up, Down, Left, Right].drain(0..4) {
            let mut n = self.clone();
            n.offset(CoordinateOffset::from_facing(face, dimensions));
            neighbors.push(n);
        }

        neighbors
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

    fn from_facing(facing: crate::r#mod::Facing, dimensions: &'a iced::Size<usize>) -> Self {
        use crate::r#mod::Facing::*;

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

    fn coordinate_facing(&self, dimensions: &iced::Size<usize>) -> Option<Coordinate> {
        if let TileContents::Agent(agent) = &self.contents {
            let mut coord = self.coord.clone();
            coord.offset(CoordinateOffset::from_facing(agent.facing, dimensions));

            return Some(coord);
        }

        None
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

struct UniverseSettings {
    reproduction_multiplier: f32,
    food_decay_multiplier: f32
}

impl Default for UniverseSettings {
    fn default() -> Self {
        Self {
            reproduction_multiplier: 1.0,
            food_decay_multiplier: 0.5
        }
    }
}

pub(crate) struct Universe {
    tiles: HashMap<Coordinate, RefCell<Tile>>,
    settings: UniverseSettings,
    pub(crate) dimensions: iced::Size<usize>
}

impl Default for Universe {
    fn default() -> Self {
        let dimensions: iced::Size<usize> = iced::Size::new(32, 32);
        Self::new(dimensions, 64, 128, None)
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
            settings: UniverseSettings::default(),
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
        // food diffusion
        'topple: loop {
            for coord in self.coordinates() {
                 self.topple(&coord);
            }

            let mut invalid = false;
            for coord in self.coordinates() {
                if let TileContents::Food(amount) = self.get(&coord).unwrap().contents {
                    if amount > 4 {
                        invalid = true;
                    }
                }
            }

            if !invalid {
                break 'topple;
            }
        }

        // births
        let mut births: Vec<Tile> = Vec::new();
        for coord in self.coordinates() {
            if let TileContents::Agent(ref mut agent) = self.get_mut(&coord).unwrap().contents {
                let adjusted_fitness = (agent.fitness as f32 * self.settings.reproduction_multiplier) as u8;
                if thread_rng().gen_range(0..=255) < adjusted_fitness {
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

        // add new births to the HashMap of tile
        for tile in births.drain(0..births.len()) {
            self.put(tile);
        }

        // perform action
        for tile in self.tiles() {
            match &tile.contents {
                TileContents::Agent(agent) => {
                    if let Some(action) = agent.resolve(&Sense::new(self, &tile)) {
                        self.perform_action(tile.coord, agent, action);
                    }
                },
                TileContents::Food(amount) => {
                    let upper_bound = (10 as f32 * self.settings.food_decay_multiplier.recip()) as u8;
                    if thread_rng().gen_range(0..=upper_bound) < *amount {
                        self.decrement_food_at(&tile.coord);
                    }
                },
                _ => {  }
            }
        }
    }

    fn perform_action(&mut self, coord: Coordinate, agent: &Agent, action: ActionType) {
        let mut action_success: Option<Coordinate> = None;

        // TODO: action_success: bool & modify coord param directly
        //       Use coord param to update agent's action history

        // TODO: Add Universe::move method that moves a tile to a new coord

        use crate::gene::ActionType::*;
        match action {
            Move => {
                // information about the cell in front of the agent
                let target = self.get(&coord).unwrap().coordinate_facing(&self.dimensions);
                if target.is_none() {
                    return;
                }

                let target = target.unwrap();
                let target_tile = match self.get(&target) {
                    Some(t) => Some(t.contents.clone()),
                    None => None
                };

                // determine whether the agent should move, eat, or rest
                let mut can_move = false;
                let mut can_eat = false;
                match target_tile {
                    Some(target_contents) => {
                        if let TileContents::Food(..) = target_contents {
                            can_eat = true;
                        }
                    },
                    None => can_move = true
                }

                // eat
                if can_eat {
                    self.decrement_food_at(&target);

                    let mut a = agent.clone();
                    a.increment_fitness();
                    self.get_mut(&coord).unwrap().contents = TileContents::Agent(a);

                    action_success = Some(coord);
                }

                // move
                if can_move {
                    let tile_contents = self.tiles.remove(&coord).unwrap().borrow().contents.clone();
                    self.put(
                        Tile::new(target, tile_contents)
                    );

                    action_success = Some(target);
                }
            },
            TurnLeft | TurnRight => {
                let mut a = agent.clone();

                // turn in the proper direction
                a.facing = match action {
                    TurnLeft => a.facing.turn_left(),
                    TurnRight => a.facing.turn_right(),
                    _ => unreachable!()
                };

                self.put(
                    Tile::new(coord, TileContents::Agent(a))
                );

                action_success = Some(coord);
            },
            Kill => {
                let target_coord = self.get(&coord).unwrap().coordinate_facing(&self.dimensions);
                if let Some(target_coord) = target_coord {
                    let target = match self.get(&target_coord) {
                        Some(t) => Some(t.contents.clone()),
                        None => None
                    };

                    if let Some(TileContents::Agent(target)) = target {
                        self.put(
                            Tile::new(target_coord, TileContents::Food(target.fitness))
                        );

                        action_success = Some(target_coord);
                    }
                }
            },
            ProduceFood => {
                // get cell in front of the agent
                let target_coord = self.get(&coord).unwrap().coordinate_facing(&self.dimensions);

                match target_coord {
                    Some(target) => {
                        if let Some(..) = self.increment_food_at(&target) {
                            action_success = Some(coord);
                        }
                    },
                    None => {  }
                }
            }
        }

        if let Some(coord) = action_success {
            // add the action to the agent's history
            let contents = self.get(&coord).unwrap().contents.clone();
            if let TileContents::Agent(mut a) = contents {
                a.add_action_to_history(action);
                self.get_mut(&coord).unwrap().contents = TileContents::Agent(a);
            }
        }
    }
}

// helper methods
impl Universe {
    pub(crate) fn tiles(&self) -> Vec<Tile> {
        self.tiles.values().cloned().map(|tile| tile.into_inner()).collect::<Vec<Tile>>()
    }

    pub(crate) fn coordinates(&self) -> Vec<Coordinate> {
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
        let prev = match self.tiles.remove(&tile.coord) {
            Some(tile) => Some(tile.into_inner()),
            None => None
        };

        self.tiles.insert(tile.coord, RefCell::new(tile));

        prev
    }

    fn topple(&mut self, coord: &Coordinate) {
        if let TileContents::Food(amount) = self.get(coord).unwrap().contents {
            if amount <= 4 {
                return;
            }
        } else {
            return;
        }

        let mut neighbor_count = 0;
        for neighbor in coord.neighbors(&self.dimensions) {
            match self.increment_food_at(&neighbor) {
                Some(..) => { neighbor_count += 1; },
                None => {  }
            }
        }

        for _ in 0..neighbor_count {
            self.decrement_food_at(coord);
        }
    }

    // Some(amount) if TileContents::Food, Some(0) if empty, None otherwise
    fn food_at(&self, coord: &Coordinate) -> Option<u8> {
        match self.get(coord) {
            Some(tile) => {
                if let TileContents::Food(current) = tile.contents {
                    Some(current)
                } else {
                    None
                }
            },
            None => {
                Some(0)
            }
        }
    }

    fn decrement_food_at(&mut self, coord: &Coordinate) {
        match self.food_at(coord) {
            Some(amount) => {
                self.tiles.remove(coord);
                if amount > 1 {
                    self.put(
                        Tile::new(coord.clone(), TileContents::Food(amount - 1))
                    );
                }
            },
            None => {  }
        }
    }

    // returns Some(amount) if food was added at the given coordinate, otherwise None
    fn increment_food_at(&mut self, coord: &Coordinate) -> Option<u8> {
        match self.food_at(coord) {
            Some(amount) => {
                if amount < 255 {
                    self.put(
                        Tile::new(coord.clone(), TileContents::Food(amount + 1))
                    );

                    Some(amount + 1)
                } else {
                    None
                }
            }, None => None
        }
    }
}

// TODO: Implement Sense struct
pub(crate) struct Sense {

}

impl Sense {
    pub(crate) fn new(_universe: &Universe, _tile: &Tile) -> Self {
        Self {

        }
    }

    pub(crate) fn get(&self, sense: &SenseType) -> f32 {
        // use crate::gene::SenseType::*;
        match sense {
            _ => thread_rng().gen_range(1..=100) as f32 / 100f32
        }
    }
}