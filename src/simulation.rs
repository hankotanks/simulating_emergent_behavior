use rand::{Rng, thread_rng};

use crate::tile;
use crate::tile::coord;
use crate::agent;
use crate::agent::gene;

pub(crate) struct SimulationSettings {
    dimensions: iced::Size<usize>,
    agents: usize,
    complexity: usize,
    seed: Option<u64>
}

impl Default for SimulationSettings {
    fn default() -> Self {
        Self {
            dimensions: iced::Size::new(32, 32),
            agents: 32,
            complexity: 64,
            seed: None
        }
    }
}

pub(crate) struct Simulation(tile::TileMap);

impl Simulation {
    pub(crate) fn new(settings: SimulationSettings) -> Self {
        let mut prng: rand::rngs::StdRng = match settings.seed {
            Some(s) => rand::SeedableRng::seed_from_u64(s),
            None => rand::SeedableRng::from_entropy()
        };

        Self( {
                let mut t = tile::TileMap::new(settings.dimensions);

                for _ in 0..settings.agents {
                    let agent = 'agent: loop {
                        match agent::Agent::from_prng(settings.complexity, &mut prng) {
                            Ok(agent) => break 'agent agent,
                            Err(..) => continue 'agent
                        }
                    };

                    'occupied: loop {
                        let coord = coord::Coord::new(
                            prng.gen_range(0..settings.dimensions.width),
                            prng.gen_range(0..settings.dimensions.height)
                        );

                        if !t.exists(coord) {
                            t.put(coord, tile::Tile::new_agent(agent));
                            break 'occupied;
                        }
                    }
                }

                t
            } )
    }

    pub(crate) fn step(&mut self) {
        // food diffusion
        'topple: loop {
            for coord in self.food() {
                if self.get(coord).should_topple() {
                    self.topple(coord);
                }
            }

            let mut invalid = false;
            self.food().drain(0..).for_each(|coord| {
                if self.get(coord).should_topple() {
                    invalid = true;
                }
            } );

            if !invalid {
                break 'topple;
            }
        }

        // handle deaths before births
        for coord in self.agents() {
            if self.should_die(coord) {
                self.kill(coord);
            }
        }

        // handle births
        for coord in self.agents() {
            if thread_rng().gen_range(0..u8::from(ux::u5::MAX))
                < u8::from(self.get(coord).get_agent().fitness) {
                let child_coord = coord.sample_offset(
                    coord::Offset::from_direction(
                        self.get(coord).get_agent().direction.opposite()),
                    &self.0.dimensions
                );

                if !self.exists(child_coord) {
                    self.get(coord).update_agent(|mut agent| {
                        agent.fitness = ux::u5::MIN;
                    } );

                    let child = self.get(coord).get_agent().reproduce();
                    if let Ok(child) = child  {
                        self.0.put(child_coord, tile::Tile::new_agent(child));
                    }
                }

            }
        }

        // agents perform actions
        for coord in self.agents() {
            if self.exists(coord) {
                if let tile::Tile::Agent(..) = self.get(coord) {
                    let action = self.get(coord).get_agent().process(&Sense::new());
                    if let Some(action) = action { // TODO: Provide Sense struct with required parameters
                        self.act(coord, action);
                    }

                }
            }
        }

        // food randomly decays
        for coord in self.food() {
            if thread_rng().gen_range(0..=tile::Tile::DIFFUSION_THRESHOLD) == self.get(coord).food() {
                self.remove_food_at(coord);
            }
        }

    }

    fn act(&mut self, mut coord: coord::Coord, action: gene::ActionType) {
        let direction = self.get(coord).get_agent().direction;
        let facing = coord.sample_offset(
            coord::Offset::from_direction(direction),
            &self.0.dimensions
        );

        let mut successful = true;

        use gene::ActionType::*;
        match action {
            Move => {
                if !self.exists(facing) {

                    let old = coord;

                    coord = self.0.walk_towards(coord, direction);

                    if old == coord {
                        successful = false;
                    }

                } else if self.0.contains_food(facing) {
                    self.remove_food_at(facing);

                    self.get(coord).update_agent(|mut agent| {
                        agent.sate();
                    } );


                }
            },
            TurnLeft | TurnRight => {
                self.get(coord).update_agent(|mut agent| {
                    agent.direction = match action {
                        TurnLeft => agent.direction.left(),
                        TurnRight => agent.direction.right(),
                        _ => unreachable!()
                    };
                } );
            },
            Kill => {
                if self.exists(facing) {
                    if let tile::Tile::Agent(..) = self.get(facing) {
                        self.kill(facing);
                    } else {
                        successful = false;
                    }
                } else {
                    successful = false;
                }
            },
            ProduceFood => {
                if !self.add_food_at(facing) {
                    successful = false;
                }
            }
        }

        self.get(coord).update_agent(|mut agent| {
            agent.acted(action, successful);
        } );
    }

    fn kill(&mut self, coord: coord::Coord) {
        if self.0.contains_agent(coord) {
            let amount = self.get(coord).get_agent().fitness;
            self.0.clear(coord);

            for _ in 0..u8::from(amount) {
                self.add_food_at(coord);
            }

            return;
        }

        panic!()
    }

    // assumes Tile is an Agent
    fn should_die(&self, coord: coord::Coord) -> bool {
        let fitness = self.get(coord).get_agent().fitness;
        let starving = self.get(coord).get_agent().starving();

        // Agents have a random chance to die if they are starving
        // Fitter creatures have a lower chance of dying
        if thread_rng().gen_range(0..u8::from(ux::u5::MAX)) > u8::from(fitness) && starving {
            return true;
        }

        false
    }

    fn topple(&mut self, coord: coord::Coord) {
        for neighbor in coord.neighbors(&self.0.dimensions) {
            self.add_food_at(neighbor);
            if self.remove_food_at(coord) {
                break;
            }
        }
    }

    // returns true if food was successfully added
    fn add_food_at(&mut self, coord: coord::Coord) -> bool {
        if self.0.contains_food(coord) {
            self.get(coord).add_food();
            return true;
        } else if !self.exists(coord) {
            self.0.put(coord, tile::Tile::new_food(1));
            return true;
        }

        false
    }

    // returns true if the tile is removed
    fn remove_food_at(&mut self, coord: coord::Coord) -> bool {
        if self.0.contains_food(coord) {
            if self.get(coord).remove_food() {
                self.0.clear(coord);
                return true;
            }

            return false;
        }

        panic!()
    }
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new(SimulationSettings::default())
    }
}

// helper methods
impl Simulation {
    pub(crate) fn get(&self, coord: coord::Coord) -> &tile::Tile {
        self.0.get(coord)
    }

    pub(crate) fn exists(&self, coord: coord::Coord) -> bool {
        self.0.exists(coord)
    }

    pub(crate) fn size(&self) -> iced::Size<usize> {
        self.0.dimensions
    }

    pub(crate) fn coords(&self) -> Vec<coord::Coord> {
        self.0.coords()
    }

    pub(crate) fn food(&self) -> Vec<coord::Coord> {
        let mut coords = self.coords();
        coords.drain(0..coords.len()).filter(|coord| {
            matches!(self.get(*coord), tile::Tile::Food(..))
        } ).collect::<Vec<coord::Coord>>()
    }

    pub(crate) fn agents(&self) -> Vec<coord::Coord> {
        let mut coords = self.coords();
        let mut coords = coords.drain(0..coords.len()).filter(|coord| {
            matches!(self.get(*coord), tile::Tile::Agent(..))
        } ).collect::<Vec<coord::Coord>>();

        coords.sort_by(|first, second| {
            let first_fitness = u8::from(self.get(*first).get_agent().fitness);
            let second_fitness = u8::from(self.get(*second).get_agent().fitness);

            first_fitness.cmp(&second_fitness)
        } );

        coords
    }
}

pub(crate) struct Sense;

impl Sense {
    pub(crate) fn new() -> Self {
        Self {  }
    }

    pub(crate) fn get(&self, _sense: &gene::SenseType) -> f32 {
        thread_rng().gen_range(0..100) as f32 / 100f32
    }
}