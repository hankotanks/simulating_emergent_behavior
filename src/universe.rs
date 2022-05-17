use std::fmt;
use std::fmt::Formatter;
use rand::Rng;

use crate::agent::Agent;

pub(crate) struct Cell {
    x: usize,
    y: usize,
    pub(crate) contents: Option<CellContents>
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cell @ ({}, {}): {}", self.x, self.y, match &self.contents {
            Some(contents) => match contents {
                CellContents::Food(amount) => format!("Food {}", amount),
                CellContents::Agent(agent) => format!("{}", agent)
            },
            None => String::from("Empty")
        })
    }
}

impl Cell {
    fn new(x: usize, y: usize) -> Self {
        Self {
            x,
            y,
            contents: None
        }
    }

    fn empty(&self) -> bool {
        if let None = self.contents {
            return true;
        }

        false
    }

    fn fill(&mut self, contents: CellContents) {
        self.contents = Some(contents);
    }

    pub(crate) fn color(&self) -> iced::Color {
        match &self.contents {
            Some(contents) => { match contents {
                CellContents::Food(amount) => iced::Color::from_rgba8(0xFF, 0x64, 0x00, *amount as f32 / 255f32),
                CellContents::Agent(..) => iced::Color::from_rgb8(0x96, 0x64, 0xFF)
            }},
            None => iced::Color::from_rgb8(0x40, 0x44, 0x4B)
        }
    }
}

#[derive(Clone)]
pub(crate) enum CellContents {
    Food(u8),
    Agent(Agent)
}

pub(crate) struct Universe {
    pub(crate) cells: Vec<Vec<Cell>>
}

impl Universe {
    pub(crate) fn new(width: usize, height: usize, agents: usize, complexity: usize, seed: Option<u64>) -> Self {
        let mut prng: rand::rngs::StdRng = match seed {
            Some(s) => rand::SeedableRng::seed_from_u64(s),
            None => rand::SeedableRng::from_entropy()
        };

        Self {
            cells: {
                let mut universe: Vec<Vec<Cell>> = (0..height).map(|y| (0..width).map(|x| Cell::new(x, y)).collect()).collect();

                for _ in 0..agents {
                    'occupied: loop {
                        let y = prng.gen_range(0..height);
                        let x = prng.gen_range(0..width);

                        if universe[y][x].empty() {
                            match Agent::from_seed(complexity, &mut prng) {
                                Ok(agent) => {
                                    universe[y][x].fill(CellContents::Agent(agent));
                                    break 'occupied;
                                },
                                Err(..) => continue 'occupied
                            }
                        }
                    }
                }

                universe
            }
        }
    }
}


// helper methods
impl Universe {
    pub(crate) fn width(&self) -> usize {
        self.cells[0].len()
    }

    pub(crate) fn height(&self) -> usize {
        self.cells.len()
    }

    pub(crate) fn get(&self, x: usize, y: usize) -> &Cell {
        &self.cells[y][x]
    }
}