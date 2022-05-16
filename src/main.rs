mod gene;
mod agent;
mod universe;

fn main() {
    let sample = agent::Agent::from_seed(64, None);
    if let Some(t) = sample.resolve() {
        println!("{:?}\n", t);
    }

    println!("{}", sample);
}