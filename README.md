# Simulating Emergent Behavior

I've found plenty of fantastic genetic algorithm implementations out there (see: [Life Simulator](https://life-simulator.netlify.app/) and [this video](https://youtu.be/N3tRFayqVtk) by David Miller). There's a number of markedly different approaches out there. Some represent creatures as a series of cells, while others keep the creature simple while instead evolving their behavior. This is my take on the latter. The goal is to turn a string of numbers (the creature's *genome*) and turn it into a series of interconnected neurons that dictate the organism's behavior based on external stimuli[^1]. Then, over many generations, mutations in the genome will lead to creatures that interact in a quasi-intelligent manner.

For example, from the following genome...  
`10000001 11000100 00000100 10000000 01010110 10101110 01100001 00000001 00100000 11010010 10000010 10000010 10011111`

...we can construct a creature with a 'brain' that looks like this:
<img src="/images/01.png" alt="a simple brain" width="50%"/>

Some assumptions can be made about this creature's behavior (it appears to dislike noise and move towards food), but the 'logic' behind its choices becomes obfuscated as the size of its brain increases.

## Evolution

The simulation runs generation by generation. At the end of each generation, the 'fitness' of each creature is assessed, and the most successful organisms produce offspring. However, these offspring are not just copies of their parent. Mutations occur frequently, and over the course of generations, better and better survival tactics emerge (in theory).

[^1]: Genes code for nuerons and the connections between them.
  Here's how nodes are processed:
  ```
  0 0 X X X X X X
    │ │ └──┬────┘
    │ │    type of nueron
    │ sense/action
    if 1, represents an internal node
    (remaining bits encode the node's bias)
  ``` 
  Connections work a little differently.
  It takes two genes to encode a connection.
  The first is the input node's index, the second is the output's.
  ```
  1 X X X X X X X
    │ └───┬─────┘
    │     node index
    inhibitory/additive connection
  ``` 
  Inhibitory connections weaken their output node's signal, additive strengthens it.