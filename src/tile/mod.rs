pub(crate) mod coord;

use std::collections::HashMap;
use std::fmt::{Debug, Formatter};

use coord::Coord;

#[derive(Clone)]
enum Tile {
    Agent(crate::agent::Agent),
    Food(u8),
    Wall
}

impl Debug for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use Tile::*;
        write!(f, "{}", match self {
            Food(amount) => format!("Food ({})", *amount),
            Agent(agent) => format!("{}", agent),
            Wall => String::from("Wall")
        } )
    }
}

struct TileMap {
    tiles: HashMap<Coord, Tile>,
    pub(crate) dimensions: iced::Size<usize>
}

impl TileMap {
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
    /// Should be used before methods like get, update & walk in situations where a Tile's presence isn't guaranteed
    pub(crate) fn exists(&self, coord: Coord) -> bool {
        if let Some(..) = self.tiles.get(&coord) {
            return true;
        }

        false
    }

    /// Provides a Tile to the closure, updates the Tile with the closure's return
    /// Can panic if the Tile does not exist at the given Coord
    pub(crate) fn update<F>(&mut self, coord: Coord, f: F) where F: Fn(Tile) -> Tile {
        let target = self.get(coord).clone();
        let target = f(target);

        self.put(coord, target);
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
}