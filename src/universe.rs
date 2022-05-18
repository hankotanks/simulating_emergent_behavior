use std::fmt;
use std::fmt::Formatter;
use iced::Point;
use rand::Rng;

use crate::agent::Agent;

#[derive(Clone)]
pub(crate) struct Cell {
    pub(crate) x: usize,
    pub(crate) y: usize,
    pub(crate) contents: Option<CellContents>
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Cell @ ({}, {}): {}", self.x, self.y, match &self.contents {
            Some(contents) => format!("{}", contents),
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

    pub(crate) fn set_contents(&mut self, contents: CellContents) {
        self.contents = Some(contents);
    }

    pub(crate) fn color(&self) -> iced::Color {
        use iced::Color;

        // TODO: This should be able to be configured
        let agent_color: Color = Color::from_rgb8(0x96, 0x64, 0xFF);
        let wall_color: Color = Color::from_rgb8(0xFF, 0xFF, 0xFF);
        let food_color: Color = Color::from_rgb8(0xFF, 0x64, 0x00);
        let default_color: Color = Color::from_rgb8(0x40, 0x44, 0x4B);

        match &self.contents {
            Some(contents) => { match contents {
                CellContents::Food(..) => food_color,
                CellContents::Agent(..) => agent_color,
                CellContents::Wall => wall_color
            }},
            None => default_color
        }
    }

    pub(crate) fn get_tooltip(&self) -> String {
        match &self.contents {
            Some(contents) => {
                format!("{} @ ({}, {})", match contents {
                    CellContents::Agent(agent) => format!("Agent ({:?})", agent.facing),
                    CellContents::Food(amount) => format!("Food: {}", amount),
                    CellContents::Wall => String::from("Wall")
                }, self.x, self.y)
            },
            None => String::from("Empty")
        }
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
    pub(crate) cells: Vec<Vec<Cell>>,
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
                let mut universe: Vec<Vec<Cell>> = (0..dimensions.height)
                    .map(|y| (0..dimensions.width).map(|x| Cell::new(x, y)).collect()).collect();

                for _ in 0..agents {
                    'occupied: loop {
                        let y = prng.gen_range(0..dimensions.height);
                        let x = prng.gen_range(0..dimensions.width);

                        if universe[y][x].empty() {
                            match Agent::from_seed(complexity, &mut prng) {
                                Ok(agent) => {
                                    universe[y][x].set_contents(CellContents::Agent(agent));
                                    break 'occupied;
                                },
                                Err(..) => continue 'occupied
                            }
                        }
                    }
                }

                universe
            },

            dimensions
        }
    }
}