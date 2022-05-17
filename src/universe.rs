use rand::Rng;
use std::fmt;
use std::fmt::Formatter;
use crate::agent::Agent;

#[derive(Clone)]
pub(crate) enum Cell {
    Empty,
    Food(usize),
    Agent(Agent)
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let c = self.clone();
        match c {
            Cell::Empty => write!(f, "{}", c),
            Cell::Food(count) => write!(f, "Food: {}", count),
            Cell::Agent(agent) => write!(f, "{}", agent)
        }
    }
}

#[derive(Clone)]
pub(crate) struct Universe(Vec<Vec<Cell>>);

// helper methods
impl Universe {
    pub(crate) fn width(&self) -> usize {
        self.0[0].len()
    }

    pub(crate) fn height(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn get(&self, x: usize, y: usize) -> &Cell {
        &self.0[y][x]
    }
}

impl Universe {
    pub(crate) fn new(dimensions: (usize, usize), agents: usize, complexity: usize, seed: Option<u64>) -> Self {
        let mut prng: rand::rngs::StdRng = match seed {
            Some(s) => rand::SeedableRng::seed_from_u64(s),
            None => rand::SeedableRng::from_entropy()
        };

        Self {
            0: {
                let mut universe = vec![vec![Cell::Empty; dimensions.1]; dimensions.0];

                for _ in 0..agents {
                    'occupied: loop {
                        let y = prng.gen_range(0..dimensions.0);
                        let x = prng.gen_range(0..dimensions.1);

                        if let Cell::Empty = universe[y][x] {
                            match Agent::from_seed(complexity, &mut prng) {
                                Ok(a) => {
                                    universe[y][x] = Cell::Agent(a);
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