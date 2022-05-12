mod gene;
mod agent;
mod universe;

use rand::Rng;

fn main() {
    for _ in 0..1 {
        let b = random_agent(1000, None);

        println!("{}", b);

        if let Some(t) = b.resolve() {
            println!("{:?}", t);
        }
    }
}

fn random_agent(genome_length: usize, seed: Option<u64>) -> agent::Agent {
    let mut prng: rand::rngs::StdRng = match seed {
        Some(s) => rand::SeedableRng::seed_from_u64(s),
        None => rand::SeedableRng::from_entropy()
    };

    let mut genome: Vec<gene::Gene> = Vec::new();
    for _ in 0..genome_length {
        genome.push(gene::Gene::new(prng.gen_range(0..=255)));
    }

    agent::Agent::new(genome)
}

fn resolve_agent_from_string(data: &str) {
    let mut b = agent::Agent::from_string(data);

    println!("{}", b);

    if let Some(t) = b.resolve() {
        println!("{:?}", t);
    }
}

// TODO: Randomly select an output when they have equal dominance
// TODO: Separate genome into its own struct
