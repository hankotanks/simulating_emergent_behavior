use std::fmt;
use std::fmt::Formatter;
use petgraph::graph;
use petgraph::Direction;
use petgraph::graph::NodeIndex;

use crate::gene::Gene;
use crate::gene::GeneParse;

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

enum _NodeType {
    Sense,
    Action,
    Internal
}

pub(crate) struct Agent {
    brain: graph::Graph<Node, bool>,
    genome: Vec<Gene>
}

impl Agent {
    pub(crate) fn new(genome: Vec<Gene>) -> Self {
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
                    let a = (*a % brain.node_count()) as u32;
                    let b = (*b % brain.node_count()) as u32;

                    brain.add_edge(NodeIndex::from(a), NodeIndex::from(b), *inverted);
                }
            }
        }

        let mut a = Self {
            brain,
            genome
        };

        a.prune();
        a.brain.shrink_to_fit();
        a
    }

    fn prune(&mut self) {
        for index in self.brain.node_indices() {
            match self.brain[index] {
                Node::Sense(..) => {
                    self.clear_walk_edges(
                        self.brain.neighbors_directed(index, Direction::Incoming).detach()
                    );
                },
                Node::Action(..) => {
                    self.clear_walk_edges(
                        self.brain.neighbors_directed(index, Direction::Outgoing).detach()
                    );
                }, _ => {  }
            }
        }

        self.prune_isolates(None);
    }

    fn prune_isolates(&mut self, size: Option<usize>) -> usize {
        let mut remove: Vec<NodeIndex> = Vec::new();
        for index in self.brain.node_indices() {
            if self.removable(index) {
                remove.push(index);
            }
        }

        for &node in remove.iter() {
            self.brain.remove_node(node);
        }

        if let Some(t) = size {
            if t != self.brain.node_count() {
                self.prune_isolates(Some(self.brain.node_count()));
            }
        } else {
            self.prune_isolates(Some(self.brain.node_count()));
        }

        self.brain.node_count()
    }

    pub(crate)fn resolve(&self) -> Option<crate::gene::ActionType> {
        let mut dominant: Option<(crate::gene::ActionType, f32)> = None;
        for index in self.brain.externals(Direction::Outgoing) {
            if let Node::Action(variant) = &self.brain[index] {
                if let Some(weight) = self.resolve_node(index, &mut Vec::new()) {
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

    fn resolve_node(&self, index: NodeIndex, history: &mut Vec<NodeIndex>) -> Option<f32> {
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
        match &self.brain[index] {
            Sense(_v) => {
                Some(1f32)
            },
            Action(_v) => {
                self.average_neighbor_resolutions_directed(index, Direction::Incoming, 1f32, history)
            },
            Internal(bias) => {
                match self.average_neighbor_resolutions_directed(index, Direction::Incoming, *bias, history) {
                    Some(t) => Some(t),
                    None => Some(*bias)
                }
            }
        }
    }

    pub(crate) fn from_string(data: &str) -> Self {
        Self::new(crate::gene::Genome::from_string(data))
    }
}

impl Agent {
    fn clear_walk_edges(&mut self, mut walk: graph::WalkNeighbors<graph::DefaultIx>) {
        'deletion: loop {
            match walk.next_edge(&self.brain) {
                Some(t) => {
                    self.brain.remove_edge(t);
                },
                None => break 'deletion
            }
        }
    }

    fn average_neighbor_resolutions_directed(&self, index: NodeIndex, dir: Direction, bias: f32, history: &mut Vec<NodeIndex>) -> Option<f32> {
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

        match self.brain.neighbors_directed(index, dir).fold((0, 0f32), |(c, sum), r| {

            if let Some(t) = self.resolve_node(r, history) {
                let mut t = t;
                if let Some(b) = edge {
                    t *= if !b { 1f32 } else { -1f32 };
                }
                (c + 1, sum + t)
            } else {
                (c, sum)
            }
        }) {
            (0, ..) => None,
            (c, sum) => Some(sum / c as f32 * bias)
        }
    }

    fn removable(&self, index: NodeIndex) -> bool {
        use Node::*;

        return match &self.brain[index] {
            Sense(..) | Internal(..) => {
                let outputs = self.brain.neighbors_directed(index, Direction::Outgoing);
                if outputs.clone().count() == 1 {
                    match outputs.clone().next() {
                        Some(t) => {
                            t == index
                        },
                        None => unreachable!()
                    }
                } else if outputs.count() == 0 {
                    true
                } else {
                    false
                }
            },
            Action(..) => {
                self.brain.neighbors_directed(index, Direction::Incoming).count() == 0

            }
        }
    }
}

impl fmt::Display for Agent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "genome {{\n    {}\n}}\n\n{}", {
            self.genome.iter().fold(String::new(), |mut c, g| {
                c.push_str(&*format!("{} ", g));
                c
            })
        }, petgraph::dot::Dot::new(&self.brain))
    }
}