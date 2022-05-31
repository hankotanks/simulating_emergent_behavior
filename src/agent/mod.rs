pub(crate) mod gene;

use std::fmt;
use std::fmt::Formatter;

use petgraph::graph;
use petgraph::graph::NodeIndex;

use rand::{Rng, thread_rng};
use rand::rngs::StdRng;

use gene::Gene;
use gene::GeneParse;

use crate::simulation::Sense;

#[derive(Debug, Clone)]
pub(crate) enum Node {
    Sense(gene::SenseType),
    Action(gene::ActionType),
    Internal(f32)
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right
}

impl Default for Direction {
    fn default() -> Self {
        use Direction::*;
        vec![Up, Down, Left, Right][thread_rng().gen_range(0..3)]
    }
}

impl Direction {
    pub(crate) fn left(&self) -> Self {
        use Direction::*;

        match self {
            Up => Left,
            Down => Right,
            Left => Down,
            Right => Up
        }
    }

    pub(crate) fn right(&self) -> Self {
        use Direction::*;

        match self {
            Up => Right,
            Down => Left,
            Left => Up,
            Right => Down
        }
    }

    pub(crate) fn opposite(&self) -> Self {
        use Direction::*;
        match self {
            Up => Down,
            Down => Up,
            Left => Right,
            Right => Left
        }
    }
}

#[derive(Clone)]
pub(crate) struct Agent {
    pub(crate) brain: graph::Graph<Node, bool>,
    pub(crate) genome: Vec<Gene>,
    pub(crate) fitness: ux::u5,
    pub(crate) direction: Direction,
    pub(crate) history: Vec<gene::ActionType>,
    pub(crate) nutrition: ux::u5
}

impl Agent {
    const HISTORY_SIZE: usize = 20;

    pub(crate) fn new(genome: Vec<Gene>) -> Result<Self, std::io::Error> {
        use GeneParse::*;
        let mut brain: graph::Graph<Node, bool> = graph::Graph::new();

        let mut edges: Vec<GeneParse> = Vec::new();
        for gene in genome.iter() {
            let parsed = gene.parse();
            match parsed {
                Sense(variant) => { brain.add_node(Node::Sense(variant)); },
                Action(variant) => { brain.add_node(Node::Action(variant)); },
                Internal(bias) => { brain.add_node(Node::Internal(bias)); },
                Connection(..) => { edges.push(parsed); }
            }
        }

        for i in 0..(edges.len() / 2) {
            if let Connection(a, inverted) = &edges[i * 2] {
                if let Connection(b, ..) = &edges[i * 2 + 1] {
                    if brain.node_count() == 0 {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid Genome"));
                    }
                    let a = (*a % brain.node_count()) as u32;
                    let b = (*b % brain.node_count()) as u32;

                    brain.add_edge(NodeIndex::from(a), NodeIndex::from(b), *inverted);
                }
            }
        }

        for index in brain.node_indices() {
            match brain[index] {
                Node::Sense(..) => {
                    let mut walk = brain.neighbors_directed(index, petgraph::Direction::Incoming).detach();
                    'sense_deletion: loop {
                        match walk.next_edge(&brain) {
                            Some(t) => {
                                brain.remove_edge(t);
                            },
                            None => break 'sense_deletion
                        }
                    }
                },
                Node::Action(..) => {
                    let mut walk = brain.neighbors_directed(index, petgraph::Direction::Outgoing).detach();
                    'action_deletion: loop {
                        match walk.next_edge(&brain) {
                            Some(t) => {
                                brain.remove_edge(t);
                            },
                            None => break 'action_deletion
                        }
                    }
                },
                _ => {}
            }
        }

        let mut agent = Self {
            brain,
            genome,
            fitness: ux::u5::MIN,
            direction: Direction::default(),
            history: Vec::new(),
            nutrition: ux::u5::MAX,
        };

        let mut retain: Vec<NodeIndex> = Vec::new();
        for index in agent.brain.node_indices() {
            if let Node::Action(..) = agent.brain[index] {
                agent.prune(index, &mut retain);
            }
        }

        agent.brain.retain_nodes(|brain, n| {
            retain.contains(&n) && {
                match &brain[n] {
                    Node::Action(..) => {
                        brain.neighbors_directed(n, petgraph::Direction::Incoming).count() != 0
                    },
                    _ => true
                }
            }
        });

        agent.brain.shrink_to_fit();

        Ok(agent)
    }

    fn prune(&mut self, index: NodeIndex, processed: &mut Vec<NodeIndex>) {
        processed.push(index);
        let mut walk = self.brain.neighbors_directed(index, petgraph::Direction::Incoming).detach();
        while let Some(t) = walk.next_node(&self.brain) {
            if !processed.contains(&t) {
                self.prune(t, processed);
            }
        }
    }

    pub(crate) fn process(&self, sense: &Sense) -> Option<gene::ActionType> {
        let mut dominant: Option<(gene::ActionType, f32)> = None;
        for index in self.brain.externals(petgraph::Direction::Outgoing) {
            if let Node::Action(variant) = &self.brain[index] {
                if let Some(weight) = self.process_node(index, sense, &mut Vec::new()) {
                    dominant = Some(
                        if let Some(highest) = dominant {
                            if weight > highest.1 {
                                (*variant, weight)
                            } else { highest }
                        } else {
                            (*variant, weight)
                        }
                    )
                }
            }
        }

        dominant.map(|t| t.0)
    }

    fn process_node(&self, index: NodeIndex, sense: &Sense, history: &mut Vec<NodeIndex>) -> Option<f32> {
        // check if the node walk is self-referential
        // internal nodes return their bias as a constant
        if history.contains(&index) {
            if let Internal(bias) = self.brain[index] {
                if self.brain.neighbors_directed(index, petgraph::Direction::Incoming).count() == 0 {
                    return Some(bias);
                }
            }
        }

        use Node::*;
        let mut bias = 1f32;
        match &self.brain[index] {
            Sense(variant) => {
                return Some(sense.get(variant))
            },
            Internal(b) => {
                bias = *b;
            }
            _ => {}
        };

        if history.contains(&index) {
            return if let Internal(bias) = self.brain[index] {
                Some(bias)
            } else {
                None
            }
        }

        // get the corresponding edge between the `index` node and its parent
        let edge = match history.last() {
            Some(&t) => {
                self.brain.find_edge(index, t).map(|k| self.brain[k])
            },
            None => None
        };

        history.push(index);

        match self.brain.neighbors_directed(index, petgraph::Direction::Incoming).fold((0, 0f32), |(c, sum), r| {
            if let Some(t) = self.process_node(r, sense, history) {
                let mut t = t;
                if let Some(b) = edge {
                    t *= if b { 1f32 } else { -1f32 };
                }
                (c + 1, sum + t)
            } else {
                (c, sum)
            }
        }) {
            (0, ..) => {
                if let Internal(..) = &self.brain[index] {
                    Some(bias)
                } else {
                    None
                }
            },
            (c, sum) => Some(sum / c as f32 * bias)
        }
    }

    pub(crate) fn reproduce(&self) -> Result<Self, std::io::Error> {
        match Self::from_string(gene::Genome::mutate(self.genome.clone())) {
            Ok(agent) => Ok(agent),
            Err(e) => Err(e)
        }
    }

    pub(crate) fn acted(&mut self, action: gene::ActionType, successful: bool) {
        if self.nutrition > ux::u5::MIN {
            self.nutrition = self.nutrition - ux::u5::new(1);
        }

        // Producing food sates the creature that made it
        if matches!(action, gene::ActionType::ProduceFood) && successful {
            self.sate();
        }

        if self.history.len() > Self::HISTORY_SIZE {
            self.history.pop();
        }

        self.history.insert(0, action)
    }

    pub(crate) fn sate(&mut self) {
        self.nutrition = ux::u5::MAX;

        if self.fitness < ux::u5::MAX {
            self.fitness = self.fitness + ux::u5::new(1);
        }
    }

    pub(crate) fn starving(&self) -> bool {
        self.nutrition == ux::u5::MIN
    }
}

impl Agent {
    pub(crate) fn from_prng(complexity: usize, prng: &mut StdRng) -> Result<Self, std::io::Error> {
        let mut genome: Vec<Gene> = Vec::new();
        for _ in 0..complexity {
            genome.push(Gene::new(prng.gen_range(0..=255)));
        }

        Self::new(genome)
    }

    pub(crate) fn from_seed(complexity: usize, seed: u64) -> Result<Self, std::io::Error> {
        let mut prng: StdRng = rand::SeedableRng::seed_from_u64(seed);

        Agent::from_prng(complexity, &mut prng)
    }

    pub(crate) fn from_string(data: String) -> Result<Self, std::io::Error> {
        Self::new(gene::Genome::from_string(data))
    }
}

impl fmt::Debug for Agent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Agent{}, facing {:?}", {
            match self.history.first() {
                Some(action) => format!(" ({:?})", action),
                None => String::default()
            }
        }, self.direction)
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}