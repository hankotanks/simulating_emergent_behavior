use std::fmt;
use std::fmt::Formatter;
use std::hash;
use std::hash::Hasher;
use rand::Rng;

use fxhash::FxHashSet;
use std::collections::hash_set::Iter;

use crate::agent::Agent;
use crate::gene::{ActionType, SenseType};

struct Color(u8, u8, u8);

impl Color {
    fn get(&self) -> [f32; 3] {
        [self.0 as f32 / 255f32, self.1 as f32 / 255f32, self.2 as f32 / 255f32]
    }
}

const AGENT_COLOR: Color = Color(0x96, 0x64, 0xFF);
const WALL_COLOR: Color = Color(0xFF, 0xFF, 0xFF);
const FOOD_COLOR: Color = Color(0xFF, 0x64, 0x00);

#[derive(Clone)]
pub(crate) struct Cell {
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) contents: CellContents
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cell @ ({}, {}): {}", self.x, self.y, self.contents)
    }
}

impl PartialEq<Self> for Cell {
    fn eq(&self, other: &Self) -> bool {
        other.x == self.x && other.y == self.y
    }
}

impl Eq for Cell {}

impl hash::Hash for Cell {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

impl Cell {
    pub(crate) fn new(x: usize, y: usize) -> Self {
        Self {
            x,
            y,
            contents: CellContents::Food(0)
        }
    }

    pub(crate) fn color(&self) -> iced::Color {
        use iced::Color;
        Color::from(match &self.contents {
            CellContents::Food(..) => FOOD_COLOR,
            CellContents::Agent(..) => AGENT_COLOR,
            CellContents::Wall => WALL_COLOR
        }.get())
    }

    pub(crate) fn get_tooltip(&self) -> String {
        format!("{} @ ({}, {})", match &self.contents {
            CellContents::Agent(agent) => format!("Agent ({:?})", agent.facing),
            CellContents::Food(amount) => format!("Food: {}", amount),
            CellContents::Wall => String::from("Wall")
        }, self.x, self.y)

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
            CellContents::Food(amount) => format!("Food {}", amount),
            CellContents::Agent(agent) => format!("Agent\n{}", agent),
            CellContents::Wall => String::from("Wall")
        })
    }
}

pub(crate) struct Universe {
    cells: FxHashSet<Cell>,
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
                let mut universe = FxHashSet::default();

                for _ in 0..agents {
                    'occupied: loop {
                        let y = prng.gen_range(0..dimensions.height);
                        let x = prng.gen_range(0..dimensions.width);

                        if !universe.contains(&Cell::new(x, y)) {
                            match Agent::from_seed(complexity, &mut prng) {
                                Ok(agent) => {
                                    let mut cell = Cell::new(x, y);
                                    cell.contents = CellContents::Agent(agent);
                                    universe.insert(cell);
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
        use CellContents::*;
        for cell in self.cells.clone().iter() {
            match &self.cells.get(cell).unwrap().contents {
                Agent(agent) => {
                    let sense = Sense::new(self, cell);
                    if let Some(action) = agent.resolve(&sense) {
                        self.perform_action(cell, action);
                    }
                },
                _ => {  }
            }
        }
    }

    fn perform_action(&mut self, cell: &Cell, action: ActionType) {
        if let CellContents::Agent(agent) = &cell.contents {
            use ActionType::*;
            match action {
                Move => {
                    if let Some(transform) = agent.facing.transform(cell.x, cell.y, self.dimensions) {
                        if let None = self.get(transform.0, transform.1) {
                            let mut n = cell.clone();

                            self.cells.remove(cell);

                            n.x = transform.0;
                            n.y = transform.1;

                            self.cells.insert(n);
                        }
                    }

                },
                TurnRight | TurnLeft => {
                    let d = agent.facing.turn(match action {
                        TurnLeft => crate::agent::Facing::Left,
                        TurnRight => crate::agent::Facing::Right,
                        _ => unreachable!()
                    });

                    let mut n = cell.clone();
                    self.cells.remove(cell);
                    let mut a = agent.clone();
                    a.facing = d;

                    n.contents = CellContents::Agent(a);

                    self.cells.insert(n);

                },
                Eat => {
                    if let Some(coord) = agent.facing.transform(cell.x, cell.y, self.dimensions) {
                        if let Some(target) = self.get(coord.0, coord.1) {
                            if let CellContents::Food(amount) = target.contents {
                                let mut n = target.clone();
                                self.cells.remove(&n);

                                let mut m = cell.clone();
                                let mut d = agent.clone();
                                d.fitness += 1;
                                m.contents = CellContents::Agent(d);

                                self.cells.insert(m);

                                if amount - 1 != 0 {
                                    n.contents = CellContents::Food(amount - 1);
                                    self.cells.insert(n);
                                }
                            }
                        }
                    }
                },
                ProduceFood => {
                    if let Some(coord) = agent.facing.transform(cell.x, cell.y, self.dimensions) {
                        if let Some(target) = self.get(coord.0, coord.1) {
                            if let CellContents::Food(amount) = target.contents {
                                if amount < 255 {
                                    let mut n = target.clone();
                                    self.cells.remove(&n);
                                    n.contents = CellContents::Food(amount + 1);
                                    self.cells.insert(n);
                                }
                            }
                        } else {
                            self.cells.insert(
                                Cell { x: coord.0, y: coord.1, contents: CellContents::Food(1) }
                            );
                        }
                    }
                },
                Kill => {
                    if let Some(coord) = agent.facing.transform(cell.x, cell.y, self.dimensions) {
                        if let Some(target) = self.get(coord.0, coord.1) {
                            if let CellContents::Agent(..) = target.contents {
                                self.cells.remove(&target.clone());
                                self.cells.insert(
                                    Cell { x: coord.0, y: coord.1, contents: CellContents::Food(1) }
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Universe {
    pub(crate) fn get(&self, x: usize, y: usize) -> Option<&Cell> {
        self.cells.get(&Cell::new(x, y))
    }

    pub(crate) fn iter(&self) -> Iter<Cell> {
        self.cells.iter()
    }
}

// TODO: Implement Sense struct
pub(crate) struct Sense {

}

impl Sense {
    pub(crate) fn new(universe: &Universe, cell: &Cell) -> Self {
        Self {

        }
    }

    pub(crate) fn get(&self, sense: &SenseType) -> f32 {
        use crate::gene::SenseType::*;
        match sense {
            _ => 1f32
        }
    }
}