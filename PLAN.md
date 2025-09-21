# Two directions

## Recreational
Benefits:
 - play with lambda calculus
 - debug fully beta-expressions
 - test beta-equality of expressions via built-in #eq operator

Requirements:
 - Strong evaluation (under lambda abstractions)

Downsides:
 - Operational semantics of current implementation are unclear (formally)
 - Might be hard to modify in future to support everything

## Actual programming
Benefits:
 - write *actually useful* programs in lambo
 - advance the language further based on demands of these programs

Requirements:
 - External data-types (numbers, strings)
 - (Weak) Call by Name is enough

Downsides:
 - "Printing" a statement doesn't reveal any useful information
 - No free variables

## End goal
The end goal is to write a self-hosted compiler.

For that we definitely need to go in the **latter direction!**

Actual steps:

## Stage 1: adapting test-cases
This can be done with existing evaluation strategy.
 - [x] Replace #eq operator usage with actual extensional equality (case-dependent) 
 - [ ] Adapt test-cases to exclude free variables
 - [ ] Drop free variables support completely
 - [x] Remove #eq operator in place of debugging function
 - [ ] Remove `EvaluationOrder`

Result: removed noise, less code.

## Stage 2: finish Call By Need implementation
 - [ ] Create a *working* version of Call By Need (doesn't have to be optimal)
 - [ ] Make sure to get it right! Recursion **has** to be supported

Result: all tests pass using Call By Need

## (Optional) Stage 2.5: Strong Normal Order
 - [ ] Refactor existing eval to be Strong Normal Order, with formalized operational semantics
 - [ ] Use Strong Normal Order for debugging purposes only

Result: useful prints, debugging superpowers


## Stage 3: Data types
 - Extend operational semantics with custom data types:
  - [ ] strings
  - [ ] numbers (CPU-optimized)

Result: language is ready to start implementing more advanced programs, like itself :D


## (Optional) Stage 4: Strong Call By Need
Result: true normal forms. Free variables are now supported again, but only in Strong version. Can be used for more advanced tests (if needed).


# Evaluation is great! Further steps towards SUPERCOMPILATION
## Weak spots
 - Assoc & Lift screw up node indexes
 - New node type for `let ... in` closure?
 - DeBrujin indexes force to clone everything (if node has multiple parents it means we can reach it with different `ClosurePath`s)
 - Feels like Thunks or other mechanism is required for proper sharing anyway...

## Ergonomics
 - Primitive Garbage Collector (stop-the-world) for easier debugging
 - Consumed node type is no longer relevant?
 - Replace all `panic!` calls to a more graceful panic with dumping debug info

## Abstract Data Types
 - Key observation: all data is CONSTRUCTED. This means all contents are frozen in the constructor call. Basically data is just a pointer back to the constructor
 - DATA IS LAZY: Most of the time contents of data will be just bound variables. They will only be substituted when actually needed
 - Improved `Data` struct - tag + list of contents

### External computations as data
 - Data may be a VALUE or a NON-value `DataType = Value | Redex(dyn fn)`
 - It's the responsibility of `dyn fn` to evaluate the parameters if needed. `ClosurePath` is therefore provided as an argument as well

### Fresh look at IO
 - IO is mostly just data ;)
 - Really shows how there's no much difference between data and external computation
