use std::fmt;
use std::fmt::Formatter;

use petgraph::graph;
use petgraph::Direction;
use petgraph::graph::NodeIndex;

use rand::{Rng, thread_rng};

use crate::gene::{ActionType, Gene};
use crate::gene::GeneParse;
use crate::universe::{Coordinate, Sense};

#[derive(Debug, Clone)]
pub(crate) enum Node {
    Sense(crate::gene::SenseType),
    Action(crate::gene::ActionType),
    Internal(f32)
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Facing {
    Up,
    Down,
    Left,
    Right
}

impl Default for Facing {
    fn default() -> Self {
        use Facing::*;
        vec![Up, Down, Left, Right][thread_rng().gen_range(0..3)].clone()
    }
}

impl Facing {
    pub(crate) fn transform(&self, coord: &Coordinate, dimensions: iced::Size<usize>) -> Option<Coordinate> {
        use Facing::*;

        return match self {
            Up => {
                if coord.y == 0 {
                    None
                } else {
                    Some(Coordinate::new(coord.x, coord.y - 1))
                }
            },
            Down => {
                if coord.y >= dimensions.height - 1 {
                    None
                } else {
                    Some(Coordinate::new(coord.x, coord.y + 1))
                }
            },
            Left => {
                if coord.x == 0 {
                    None
                } else {
                    Some(Coordinate::new(coord.x - 1, coord.y))
                }
            },
            Right => {
                if coord.x >= dimensions.width - 1 {
                    None
                } else {
                    Some(Coordinate::new(coord.x + 1, coord.y))
                }
            }
        }
    }

    pub(crate) fn turn_left(&self) -> Self {
        use Facing::*;

        match self {
            Up => Left,
            Down => Right,
            Left => Down,
            Right => Up
        }
    }

    pub(crate) fn turn_right(&self) -> Self {
        use Facing::*;

        match self {
            Up => Right,
            Down => Left,
            Left => Up,
            Right => Down
        }
    }

    pub(crate) fn opposite(&self) -> Self {
        use Facing::*;
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
    brain: graph::Graph<Node, bool>,
    genome: Vec<Gene>,
    pub(crate) fitness: u8,
    pub(crate) facing: Facing,
    pub(crate) last_action: Option<ActionType>
}

impl Agent {
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
            if let GeneParse::Connection(a, inverted) = &edges[i * 2] {
                if let GeneParse::Connection(b, ..) = &edges[i * 2 + 1] {
                    if brain.node_count() == 0 {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, "Invalid Genome"));
                    }
                    let a = (*a % brain.node_count()) as u32;
                    let b = (*b % brain.node_count()) as u32;

                    brain.add_edge(NodeIndex::from(a), NodeIndex::from(b), *inverted);
                }
            }
        }

        let mut a = Self {
            brain,
            genome,
            fitness: 0u8,
            facing: Facing::default(),
            last_action: None
        };

        a.prune();
        a.brain.shrink_to_fit();
        Ok(a)
    }

    fn prune(&mut self) {
        for index in self.brain.node_indices() {
            match self.brain[index] {
                Node::Sense(..) => {
                    let mut walk = self.brain.neighbors_directed(index, Direction::Incoming).detach();
                    'sense_deletion: loop {
                        match walk.next_edge(&self.brain) {
                            Some(t) => {
                                self.brain.remove_edge(t);
                            },
                            None => break 'sense_deletion
                        }
                    }
                },
                Node::Action(..) => {
                    let mut walk = self.brain.neighbors_directed(index, Direction::Outgoing).detach();
                    'action_deletion: loop {
                        match walk.next_edge(&self.brain) {
                            Some(t) => {
                                self.brain.remove_edge(t);
                            },
                            None => break 'action_deletion
                        }
                    }
                }, _ => {  }
            }
        }

        let mut retain: Vec<NodeIndex> = Vec::new();
        for index in self.brain.node_indices() {
            if let Node::Action(..) = self.brain[index] {
                self.prune_isolates(index, &mut retain);
            }
        }

        self.brain.retain_nodes(|brain, n| {
            retain.contains(&n) && {
                match &brain[n] {
                    Node::Action(..) => {
                        if brain.neighbors_directed(n, Direction::Incoming).count() == 0 {
                            false
                        } else {
                            true
                        }
                    },
                    _ => true
                }
            }
        } );
    }

    fn prune_isolates(&mut self, index: NodeIndex, processed: &mut Vec<NodeIndex>) {
        processed.push(index);
        let mut walk = self.brain.neighbors_directed(index, Direction::Incoming).detach();
        loop {
            match walk.next_node(&self.brain) {
                Some(t) => {
                    if !processed.contains(&t) {
                        self.prune_isolates(t, processed);
                    }
                },
                None => break
            }
        }
    }

    pub(crate)fn resolve(&self, sense: &Sense) -> Option<crate::gene::ActionType> {
        let mut dominant: Option<(crate::gene::ActionType, f32)> = None;
        for index in self.brain.externals(Direction::Outgoing) {
            if let Node::Action(variant) = &self.brain[index] {
                if let Some(weight) = self.resolve_node(index, sense, &mut Vec::new()) {
                    dominant = Some (
                        if let Some(highest) = dominant {
                            if weight > highest.1 {
                                (variant.clone(), weight)
                            } else { highest }

                        } else {
                            (variant.clone(), weight)
                        }
                    )
                }
            }
        }

        match dominant {
            Some(t) => Some(t.0),
            None => None
        }
    }

    fn resolve_node(&self, index: NodeIndex, sense: &Sense, history: &mut Vec<NodeIndex>) -> Option<f32> {
        // check if the node walk is self-referential
        // internal nodes return their bias as a constant
        if history.contains(&index) {
            if let Internal(bias) = self.brain[index] {
                if self.brain.neighbors_directed(index, Direction::Incoming).count() == 0 {
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
            return if let Node::Internal(bias) = self.brain[index] {
                Some(bias)
            } else {
                None
            }
        }

        // get the corresponding edge between the `index` node and its parent
        let edge = match history.last() {
            Some(&t) => {
                match self.brain.find_edge(index, t) {
                    Some(k) => {
                        Some(self.brain[k])
                    },
                    None => None
                }
            },
            None => None
        };

        history.push(index);

        match self.brain.neighbors_directed(index, Direction::Incoming).fold((0, 0f32), |(c, sum), r| {
            if let Some(t) = self.resolve_node(r, sense, history) {
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

    pub(crate) fn reproduce(&self) -> String {
        crate::gene::Genome::mutate(self.genome.clone())
    }

    pub(crate) fn from_string(data: String) -> Result<Self, std::io::Error> {
        Self::new(crate::gene::Genome::from_string(data))
    }

    pub(crate) fn from_seed(complexity: usize, prng: &mut rand::rngs::StdRng) -> Result<Self, std::io::Error> {
        let mut genome: Vec<Gene> = Vec::new();
        for _ in 0..complexity {
            genome.push(Gene::new(prng.gen_range(0..=255)));
        }

        Self::new(genome)
    }

    pub(crate) fn get_digraph(&self) -> String {
        format!("{}", petgraph::dot::Dot::new(&self.brain))
    }

    pub(crate) fn get_genome_string(&self) -> String {
        crate::gene::Genome::get(self.genome.clone())
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Agent{}, facing {:?}", {
            match &self.last_action {
                Some(action) => format!(" ({:?})", action),
                None => String::from("")
            }
        }, self.facing)
    }
}