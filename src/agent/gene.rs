use std::fmt;
use std::fmt::Formatter;

use rand::{Rng, thread_rng};

use strum::IntoEnumIterator;

#[derive(Clone)]
pub(crate) struct Gene(pub(crate) u8);

impl Gene {
    pub(crate) fn new(data: u8) -> Self {
        Gene(data)
    }

    pub(crate) fn parse(&self) -> GeneParse {
        use GeneParse::*;

        if Gene::get_bit(self.0, 7) {
            Connection(
                Gene::get_bit_range(self.0, 0..6) as usize,
                Gene::get_bit(self.0, 6)
            )
        } else if Gene::get_bit(self.0, 6) {
            Internal(
                Gene::get_bit_range(self.0, 0..6) as f32 / 32f32
            )
        } else {
            let index = Gene::get_bit_range(self.0, 0..5) as usize;
            if Gene::get_bit(self.0, 5) {
                Action(ActionType::iter().nth(index % ActionType::iter().count()).unwrap())
            } else {
                Sense(SenseType::iter().nth(index % SenseType::iter().count()).unwrap())
            }
        }
    }

    pub(crate) fn mutate(&mut self) {
        self.0 ^= 1u8.rotate_left(thread_rng().gen_range(0..8));
    }

    pub(crate) fn from_string(data: &str) -> Result<Self, std::io::Error> {
        match u8::from_str_radix(data, 2) {
            Ok(d) => Ok(Gene::new(d)),
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, ""))
        }
    }
}

// some helper functions
impl Gene {
    fn get_bit(data: u8, index: usize) -> bool {
        data & (1 << index) != 0
    }

    fn get_ls_bits(data: u8, bits: usize) -> u8 {
        let mut d: u8 = 0b0;
        for i in 0..bits {
            match Gene::get_bit(data, i) {
                true => d += 1u8.rotate_left(i as u32),
                false => continue,
            }
        }

        d
    }

    fn get_bit_range(data: u8, range: std::ops::Range<usize>) -> u8 {
        let v = data >> range.start;
        Gene::get_ls_bits(v, range.end - range.start)
    }
}

impl fmt::Display for Gene {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:08b}", self.0)
    }
}

pub(crate) struct Genome;

impl Genome {
    const MUTATION_FREQUENCY: f32 = 0.15;

    pub(crate) fn mutate(mut genome: Vec<Gene>) -> String {
        if thread_rng().gen_range(0..100) as f32 / 100f32 < Self::MUTATION_FREQUENCY {
            if thread_rng().gen_bool(0.5f64) {
                genome.push(Gene::new(thread_rng().gen_range(0..=255)));
            } else {
                genome.remove(thread_rng().gen_range(0..genome.len()));
            }

        } else {
            let length = genome.len();
            for _ in 0..(length as f32 * Self::MUTATION_FREQUENCY) as usize {
                genome[thread_rng().gen_range(0..length)].mutate();
            }
        }

        Genome::get(genome)
    }

    pub(crate) fn get(genome: Vec<Gene>) -> String {
        Self::get_with_delim(genome, " ")
    }

    pub(crate) fn get_with_delim(genome: Vec<Gene>, delim: &str) -> String {
        genome.iter().fold("".to_owned(), |mut genome: String, current| {
            genome.push_str(&*format!("{}{}", current, delim));
            genome
        }).trim_end().to_string()
    }

    pub(crate) fn from_string(data: String) -> Vec<Gene> {
        let mut genome: Vec<Gene> = Vec::new();
        for g in data.split(' ') {
            if let Ok(gene) = Gene::from_string(g) { genome.push(gene) }
        }

        genome
    }
}

#[derive(Debug, Clone)]
pub(crate) enum GeneParse {
    Sense(SenseType),
    Action(ActionType),
    Internal(f32),
    Connection(usize, bool),
}

#[derive(Debug, Copy, Clone, strum_macros::EnumIter)]
pub(crate) enum SenseType {
    Blocked,
    Agent,
    AgentDensity,
    Food,
    FoodDensity,
    Direction
}

#[derive(Debug, Copy, Clone, strum_macros::EnumIter)]
pub(crate) enum ActionType {
    Move,
    TurnLeft,
    TurnRight,
    Kill,
    ProduceFood,
}