pub(crate) mod coord;

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::fmt;
use std::collections::HashMap;

use coord::Coord;
use crate::agent::Agent;

#[derive(Clone)]
pub(crate) enum Tile {
    Agent(RefCell<Agent>),
    Food(Cell<u8>)
}

impl Tile {
    pub(crate) const DIFFUSION_THRESHOLD: u8 = 4; // food diffuses above this value

    pub(crate) fn new_agent(agent: Agent) -> Tile {
        Self::Agent(RefCell::new(agent))
    }

    pub(crate) fn food(&self) -> u8 {
        if let Self::Food(amount) = self {
            return amount.get();
        }

        panic!()
    }

    pub(crate) fn new_food(amount: u8) -> Tile {
        Self::Food(Cell::new(amount))
    }

    pub(crate) fn add_food(&self) {
        if let Self::Food(amount) = self {
            amount.set(amount.get() + 1);
            return;
        }

        panic!()
    }

    pub(crate) fn should_topple(&self) -> bool {
        self.food() > Self::DIFFUSION_THRESHOLD
    }

    /// Removes food, returns true if empty
    pub(crate) fn remove_food(&self) -> bool {
        if let Self::Food(amount) = self {
            return if amount.get() == 1 {
                true
            } else {
                amount.set(amount.get() - 1);
                false
            }
        }

        panic!()
    }

    pub(crate) fn get_agent(&self) -> Ref<'_, Agent> {
        if let Self::Agent(agent) = self {
            return agent.borrow();
        }

        panic!()
    }

    pub(crate) fn update_agent<F>(&self, f: F) where F: Fn(RefMut<'_, Agent>) {
        if let Self::Agent(agent) = self {
            f(agent.borrow_mut());
            return;
        }

        panic!()
    }
}

impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Tile::*;
        write!(f, "{}", match self {
            Food(amount) => format!("Food ({})", amount.get()),
            Agent(..) => format!("{}", self.get_agent())
        } )
    }
}

pub(crate) struct TileMap {
    tiles: HashMap<Coord, Tile>,
    pub(crate) dimensions: iced::Size<usize>
}

impl TileMap {
    /// Create a new TileMap of a given Size
    pub(crate) fn new(dimensions: iced::Size<usize>) -> Self {
        Self {
            tiles: HashMap::new(),
            dimensions
        }
    }

    /// Puts a Tile at a given Coord.
    /// If a tile was previously present, returns it, otherwise None
    pub(crate) fn put(&mut self, coord: Coord, tile: Tile) -> Option<Tile> {
        self.tiles.insert(coord, tile)
    }

    /// Gets a reference to the Tile at a given Coord
    /// Panics if the Tile does not exist
    pub(crate) fn get(&self, coord: Coord) -> &Tile {
        match self.tiles.get(&coord) {
            Some(tile) => tile,
            None => panic!()
        }
    }

    /// Returns true if a Tile is present at the Coord
    /// Should be used before methods like get & walk in situations where a Tile's presence isn't guaranteed
    pub(crate) fn exists(&self, coord: Coord) -> bool {
        if let Some(..) = self.tiles.get(&coord) {
            return true;
        }

        false
    }

    pub(crate) fn contains_agent(&self, coord: Coord) -> bool {
        if !self.exists(coord) {
            return false;
        }

        matches!(self.get(coord), Tile::Agent(..))
    }

    pub(crate) fn contains_food(&self, coord: Coord) -> bool {
        if !self.exists(coord) {
            return false;
        }

        matches!(self.get(coord), Tile::Food(..))
    }

    pub(crate) fn clear(&mut self, coord: Coord) -> Option<Tile> {
        self.tiles.remove(&coord)
    }

    /// Applies an Offset, one step at a time by using Offset::signum
    /// The walk is halted if it is interrupted by an occupied Tile
    /// Returns the walk's termination Coord
    pub(crate) fn walk(&mut self, mut coord: Coord, offset: coord::Offset) -> Coord {
        match self.tiles.remove(&coord) {
            Some(tile) => {
                // get the new Coord and put the Tile at the new location
                self.walk_by_tiles(&mut coord, offset);
                self.put(coord, tile);
            },
            None => panic!()
        }

        // return the new Coord
        coord
    }

    /// Simple wrapper for TileMap::walk that accepts a direction instead of an Offset
    pub(crate) fn walk_towards(&mut self, coord: Coord, direction: crate::agent::Direction) -> Coord {
        self.walk(
            coord,
            coord::Offset::from_direction(direction)
        )
    }

    // Helper function for TileMap::walk
    fn walk_by_tiles(&mut self, coord: &mut Coord, mut offset: coord::Offset) {
        // update the Coord
        coord.apply_offset(offset.signum(), &self.dimensions);

        // return if the Offset is empty
        // or if the corresponding Tile is occupied
        if offset.blank() || self.exists(*coord) {
            return;
        }

        // recurse
        self.walk_by_tiles(coord, offset)
    }

    pub(crate) fn coords(&self) -> Vec<Coord> {
        self.tiles.keys().cloned().collect::<Vec<Coord>>()
    }
}