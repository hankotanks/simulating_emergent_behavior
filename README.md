# Simulating Emergent Behavior

## Genes

Genes are 8-bit unsigned integers. 
A genome, or collection of genes, is the blueprint for an agent's brain. 
This brain consists of nodes and connections between them. 
Genes define both.

The gene's most significant bit determines what it will represent.

### Nodes

Genes can encode three types of nodes. Each serves a purpose, and in general are arranged from Sensory to Internal to Action.

#### Sensory

Generates a value based on the world around it.
Ignores inputs.

```
 0 0 0 X X X X X
       └──┬────┘
          Defines which sense it represents
```

#### Action

Averages its inputs to produce a weight that represents the desirability of the action. 
Nodes of this type are calculated each turn and agent takes the action with the highest weight.

```
 0 0 1 X X X X X
       └──┬────┘
          Defines which action it represents
```

#### Internal

Each of these nodes has a 'bias' value associated with it. 
After averaging its inputs, the internal node multiplies the result by its bias and outputs it.

```
 0 1 X X X X X X
     └────┬────┘
          Represents the node's bias
           The value is normalized as a float between 0 and 2.
```

### Connections

Connections are links between nodes.
When a gene's most significant bit is 1, it represents a connection.
Each of these genes encode one side of the connection, so they are read in pairs.

```
 [ 1 X X X X X X X ]    [ 1 _ X X X X X X ]
     │ └────┬────┘          │ └────┬────┘
     │     Input index      │     Output index
     Connection sign        Ignored
```

Note that these genes don't need to be adjacent in the genome.
Connection indices are reduced to correspond to a node in the array.

#### Additive or subtractive
The connection sign, indicated by the second most significant bit of the input gene, determines if the connection is additive or subtractive. 
Negative connections inhibit the signal they propagate.