# Lambo - Lambda Calculus Compiler
> All code-blocks from this README file are actually used as an input to the main program

## Basics
The language is designed to be very minimal, with minimal changes on top of OG lambda calculus.

Expression on each line is automatically evaluated and run.

Each Lambo expression goes through 2 phases: Evaluation and Runtime.
 - Evaluation: expression is evaluated (reduced) without producing any side-effects. Deterministic; all functions are **pure** at evaluation phase.
 - Runtime: runs evaluated expression **with** side-effects (IO). It is **impure** and **non-deterministic**, because the output entirely depends on the external factors (e.g what you put into console at runtime). More on Runtime later in this README.

The output is structured like this:
`$   <EXPRESSION>`

`=>  <EVALUATION_RESULT>`

`==> <RUNTIME_RESULT>`

## Evaluation order
Many (imperative) programming languages use Strict (Applicative) evaluation order: when function call happens, arguments are evaluated before getting passed to a function. E.g in JavaScript, `f(x)` will first evaluate `x`, and only then pass the result to `f` as an argument.

Lambo uses Call-by-Name evaluation order. It is a variant of Normal (Non-Strict) evaluation order, where even function body is not reduced until it's called. This is also known as Lazy evaluation. In short, if the value is not directly used, it won't be evaluated. Lazy evaluation order is needed to be able to represent infinite structures and recursion, like Y combinator.

## Variable declaration
This is only a syntax sugar. Lambda Calculus does not have named variables, but you can
emulate them via argument names. Writing `let <name> <expr>` simply wraps 
every evaluation below it into a closure providing `<expr>` as a named argument.

Every expression below will be transformed into `(λ<variable_name>.<original>) <variable_expr>`.

```js
// Identity function
let id λx.x
```

### Assert
Let's build our first useful function: `assert_eq`. It will take two input
values and throw an error if they are not beta-equivalent. `#io_throw` here is a special beast that we will touch in later sections.

```js
let assert λx.x PASS (#io_throw FAIL)

assert λx.λy.x
// => PASS
```

## Church encoding
Below follows an example of implementating Church encoding in Lambo.

### Boolean logic
```js
let true λx.λy.x
let false λx.λy.y

let not λbool.bool false true
let =bool λa.λb.a b (not b)

assert (=bool (not false) true)

let and λp.λq.((p q) p)
assert (=bool (and false UNKNOWN) false)
```

### Pairs
Pair acts like a container holding two values.
```js
let pair λx.λy.λf.f x y
let pair_first λp.p true
let pair_second λp.p false
```

### Arithmetic
N-th Church number is a function that is essentially "Repeat N times".
```js
// Fun fact: this is actually equivalent to "false"
let 0 λf.λx.x

assert (not 0)

let =0 \n.n (\x.false) true

assert (=0 0)

let succ λn.λf.λx.(f ((n f) x))
let 1 (succ 0)
let 2 (succ 1)


// Shift-and-increment function: (m, n) -> (n, n + 1)
let Φ λx.pair (pair_second x) (succ (pair_second x))
// Easy to define predecessor function using shift-and-increment
let pred λn.pair_first (n Φ (pair 0 0))

// A + B is A, with "succ" function applied B times
let + λa.λb.((b succ) a)
let - λa.λb.((b pred) a)

let leq \a.\b.=0 (- a b)
let =num \a.\b.(and (leq a b) (leq b a))

assert (=num (succ (succ 0)) 2)
assert (=num (2 succ 0) 2)

assert (=num (pred 0) 0)
assert (=num (pred 1) 0)
assert (=num (pred 2) 1)


// A * B is (+ A) function applied B times to 0
let * λa.λb.((b (+ a)) 0)
let double (* 2)

// A ^ B is (* A) function applied B times to 1
let ^ λa.λb.((b (* a)) 1)
let square (^ 2)

assert (=num (double 2) (square 2))

let 4 (double 2)
let 8 (double 4)
let 16 (double 8)
let 32 (double 16)
let 64 (double 32)

assert (=num ((+ ((+ 1) 2)) 1) 4)
assert (=num ((+ ((+ 2) 4)) 2) 8)
assert (=num ((+ ((+ 4) 8)) 4) 16)
assert (=num (square 4) 16)
```

## Recursion
Achieving recursion proves Turing-completeness of the language.
```js
// The famous Y-combinator
[let Y λf.
    (λx.f (x x))
    (λx.f (x x))]

[let fact Y λf.λn.
    (=0 n)
    1
    (n | pred | f | (* n))]

assert (=num (fact 4) (+ 16 8))
// assert (=num (fact (succ 4)) (64 | (+ 32) | (+ 16) | (+ 8)))
```

## Runtime
Lambo has a built-in IO monad that describes side-effectful actions. From evaluator point of view, IOs are just values (aka irreducible expressions, aka normalized expressions).

The sole purpose of Runtime is to **unwrap** the IO monad that you might constuct. This **unwrap** procedure is where all the fun happens. You can not invoke **unwrap** manually, only Runtime can.

Lambo does not have types, but for a second let's imagine they exist. Runtime gives you the following tools for constructing and operating IO:
 - `#io_pure` of type `x -> IO`. A function that takes X and returns an IO monad. When Runtime unwraps this monad, program will return the contained value `x` without any side-effects.
 - `#io_print` of type `x -> IO`. A function that takes X and returns an IO monad. When Runtime unwraps this monad, program will print the contained value and return it.
 - `#io_throw` of type `x -> IO`. A function that takes X and returns an IO monad. When Runtime unwraps this monad, program will panic and print the thrown value.
 - `#io_read` of type `IO`. This is NOT a function, just an IO. When Runtime unwraps this monad, program will read a lambda expression from STDIN and return it.
 - `#io_flatmap` of type `IO -> ((x -> IO) -> IO)`. A function that takes two arguments: IO and a function `transform` that maps arbitrary value `x` to `IO`. The result is another IO. When Runtime unwraps this, it will unwrap the first IO, pass it's value to the transform function, and unwrap the final IO.

Please note that before Runtime gets into play, evaluator will treat all these values just as any other variables.

```js
// Reads two expressions from STDIN and prints the result of equality check
[let program #io_flatmap #io_read \x.
            #io_flatmap #io_read \y.
            (#io_print (= x y) )]

// ^ program has "type" IO, meaning you can actually run it with all side effects

// A funny one with recursion: keep reading from STDIN until the user inputs true
[let program Y λf.
    #io_flatmap #io_read λx.
    (= true x)
        (#io_pure DONE)
        (#io_flatmap (#io_print PLEASE_GIVE_TRUE) (\_.f))]

```

## More monads: Option
Option represents a potential absense of value.
```js
let some     λx.λs.λn.s x
let none        λs.λn.n

let option_flatmap   λoption.λtransform.option transform option
let option_map       λoption.λtransform.option (λx.x | transform | some) option
let option_or        λoption.λdefault.option some default 
let option_unwrap    λoption.option id (#io_throw EMPTY_OPTION)
let option_unwrap_or λoption.λdefault.option id default

// These 2 are equivalent
assert (=num (option_unwrap (option_map (some 2) double)) 4)
assert (=num ((option_map (some 2) double) | option_unwrap) 4)

assert (=num (option_unwrap (option_or none (some 1))) 1)
assert (=num (option_unwrap (option_or (some 2) (some 1))) 2)

assert (=num (option_unwrap_or none 2) 2)

// This will panic: (option_unwrap none)
```

## Convenience: Streams and Folds
The idea of `fold_stream` is to consume arbitrarily large stream of Options, accumulating the result. It's *not quite* the stream in usual sense, but you get the idea.

Instead of writing `(+ (+ (+ 4 1) 2) 1)` we can be a bit more fancy: `stream_sum (some 4) (some 1) (some 2) (some 1) none`, and this can be generalized to other operations.
```js

// Keeps applying combine function until encounters first None. Returns accumulated result
[let fold_stream λcombine.λinit.
    (Y λf.λacc.λoption.
        option (\x.(combine x acc) | f) acc
    ) init]

let stream_sum (fold_stream + 0)

assert (=num (stream_sum none) 0)
[assert (=num
    (stream_sum (some 4) (some 1) (some 2) (some 1) none)
    (4 | (+ 1) | (+ 2) | (+ 1))
)]
//                ^ real change of behavior                       ^ syntax sugar (x | f1 | f2) = (f2 (f1 x))
```

## Binary number constructor (wtf?)
This is my in-house creation. Don't judge!

Works very similar to fold_stream above, but with the help of the extra counter is able to decode binary numbers.
Currently doesn't use Option to not clutter syntax too much. Ideally we would zip our stream of booleans with the stream of natural numbers, and then fold it easily.
```js
// Keeps accumulating boolean values until you give it END. Returns the number :)
[let binary
    with pow_2 λn.(n double 1) in
    (Y λf.λi.λacc.λoption.
        option
        (\bool.
            f 
            (succ i) // increment i
            (bool (+ acc (pow_2 i)) acc) // update acc
        )
        acc
    )
    0 0]

[assert (=num
    (binary (some true) (some false) (some true) (some true) none)
    (1 | (+ 4) | (+ 8))
)]
```
