pub(crate) mod coord;

use std::fmt;
use std::cell;
use std::collections::HashMap;

use coord::Coord;

use crate::agent::Agent;

#[derive(Clone)]
pub(crate) enum Tile {
    Agent(cell::RefCell<Agent>),
    Food(cell::Cell<u8>)
}

impl Tile {
    /// Creates a new Tile with the provided Agent.
    pub(crate) fn new_agent(agent: Agent) -> Tile {
        Self::Agent(cell::RefCell::new(agent))
    }

    /// Gets a reference to the Agent stored in this Tile.
    ///
    /// # Panics
    /// If the Tile does not contain an Agent.
    pub(crate) fn agent(&self) -> cell::Ref<'_, Agent> {
        if let Self::Agent(agent) = self {
            return agent.borrow();
        }

        panic!()
    }

    /// Provides a mutable reference to the Tile's Agent, which can be modified through a closure.
    ///
    /// # Panics
    /// If the Tile does not contain an Agent.
    pub(crate) fn update_agent<F>(&self, f: F) where F: Fn(cell::RefMut<'_, Agent>) {
        if let Self::Agent(agent) = self {
            f(agent.borrow_mut());
            return;
        }

        panic!()
    }
}

impl Tile {
    /// When a Tile's food density exceeds the DIFFUSION_THRESHOLD, it spreads into neighboring Tiles.
    pub(crate) const DIFFUSION_THRESHOLD: u8 = 4; // food diffuses above this value

    /// Creates a new Tile with Food in the given density.
    pub(crate) fn new_food(density: u8) -> Tile {
        Self::Food(cell::Cell::new(density))
    }

    /// Gets the density of Food in the given Tile.
    ///
    /// # Panics
    /// If the Tile does not contain food.
    pub(crate) fn food(&self) -> u8 {
        if let Self::Food(density) = self {
            return density.get();
        }

        panic!()
    }

    /// Add food to the Tile.
    ///
    /// # Panics
    /// If the Tile does not contain food.
    pub(crate) fn add_food(&self) {
        if let Self::Food(amount) = self {
            amount.set(amount.get() + 1);
            return;
        }

        panic!()
    }

    /// Returns true if the Tile's food density is above the DIFFUSION_THRESHOLD.
    ///
    /// # Panics
    /// When Tile::food panics.
    pub(crate) fn should_diffuse(&self) -> bool {
        self.food() > Self::DIFFUSION_THRESHOLD
    }

    /// Removes food from the Tile.
    /// Returns true if the food's density if 0, otherwise false.
    ///
    /// # Panics
    /// If the tile does not contain food.
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
}

// Tile implements Debug.
// Provides a quick summary of the tile's contents, but does not display genome/digraph/etc..
impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Tile::*;
        write!(f, "{}", match self {
            Food(amount) => format!("Food ({})", amount.get()),
            Agent(..) => format!("{}", self.agent())
        } )
    }
}

pub(crate) struct TileMap {
    tiles: HashMap<Coord, Tile>,
    pub(crate) dimensions: iced::Size<usize>
}

impl TileMap {
    /// Create a new TileMap of a given Size.
    pub(crate) fn new(dimensions: iced::Size<usize>) -> Self {
        Self {
            tiles: HashMap::new(),
            dimensions
        }
    }

    /// Puts a Tile at a given Coord.
    /// If a tile was previously present, returns it, otherwise None.
    pub(crate) fn put(&mut self, coord: Coord, tile: Tile) -> Option<Tile> {
        self.tiles.insert(coord, tile)
    }

    /// Gets a reference to the Tile at a given Coord.
    ///
    /// # Panics
    /// If the Coord is not a key to a Tile in the TileMap.
    pub(crate) fn get(&self, coord: Coord) -> &Tile {
        match self.tiles.get(&coord) {
            Some(tile) => tile,
            None => panic!()
        }
    }

    /// Returns true if a Tile is present at the Coord.
    /// Used to validate the existence of a Tile before calling methods that can panic!
    pub(crate) fn exists(&self, coord: Coord) -> bool {
        if let Some(..) = self.tiles.get(&coord) {
            return true;
        }

        false
    }

    /// Returns true if the given Coord contains a Tile::Agent.
    pub(crate) fn contains_agent(&self, coord: Coord) -> bool {
        if !self.exists(coord) {
            return false;
        }

        matches!(self.get(coord), Tile::Agent(..))
    }

    /// Returns true if the given Coord contains food.
    pub(crate) fn contains_food(&self, coord: Coord) -> bool {
        if !self.exists(coord) {
            return false;
        }

        matches!(self.get(coord), Tile::Food(..))
    }

    /// Remove a Tile from the TileMap.
    /// Returns the removed Tile, if it was present.
    /// Otherwise, returns None.
    pub(crate) fn clear(&mut self, coord: Coord) -> Option<Tile> {
        self.tiles.remove(&coord)
    }

    /// Applies an Offset, one step at a time by using Offset::signum.
    /// The walk is halted if it is interrupted by an occupied Tile.
    /// Returns the walk's termination Coord.
    ///
    /// # Panics
    /// If the provided Coord does not contain a Tile.
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

    /// Simple wrapper for TileMap::walk that accepts a direction instead of an Offset.
    ///
    /// # Panics
    /// When TileMap::walk panics!
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

    /// Returns a vector of all Coords in the TileMap
    pub(crate) fn coords(&self) -> Vec<Coord> {
        self.tiles.keys().cloned().collect::<Vec<Coord>>()
    }
}