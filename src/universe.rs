use rand::Rng;
use crate::agent::Agent;

#[derive(Clone)]
enum Cell {
    Empty,
    Food(usize),
    Agent(Agent)
}

pub(crate) struct Universe(Vec<Vec<Cell>>);

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