# Lambo
Lambo is a Lambda Calculus based language with minimal sugar on top.

Main features:
 - Lazy evaluation (Call-by-Need)
 - ADT (Algebraic Data Types)
 - IO (Haskell-like)
 - Native arithmetic (`u64`)

This repo contains Rust interpreter for Lambo.

Right now Lambo can already be used to:
 - solve Advent of Code problems in a reasonable time (~2s for the first problem of 2025)
 - create lambda-calculus interpreter within itself
 - fuck around and have fun!

## Examples
Examples can be found in [benchmarks.lambo](./benches/benchmarks.lambo)

## NeoVim "integration"
This repo provides an additional [nvim.lua](./.nvim.lua) file with syntax highlight (OCaml-based) and `:LamboRun` comamnd for faster debugging. To load this config automatically:
```lua
vim.opt.exrc = true
vim.opt.secure = true
```

## Evaluation order
Many (imperative) programming languages use Strict (Applicative) evaluation order: when function call happens, arguments are evaluated before getting passed to a function. E.g in JavaScript, `f(x)` will first evaluate `x`, and only then pass the result to `f` as an argument.

Lambo uses Call-by-Need evaluation order (aka Lazy evaluation). It is a variant of Normal (Non-Strict) evaluation order, where even function body is not reduced until it's called. In short, if the value is not directly used, it won't be evaluated. Lazy evaluation order is needed to be able to represent infinite structures (e.g infinite list of prime numbers) and recursion in general (Y combinator, loops).

## Syntax sugar
### Functions of N arguments
In lambda calculus all functions take 1 argument. If you want more arguments, use currying (`\a.\b.\c.a b c`).
Special syntax `\a b c.a b c` is just a shorthand to represent currying.

### Variable declaration
Lambda Calculus does not have named variables, but you can
emulate them via argument names. Writing `let <name> <value> in <expr>` simply wraps 
every evaluation below it into a closure providing `<value>` as a named argument.

Every expression below will be transformed into `(λ<name>.<expr>) <value>` (although current imlementation has a handy `Closure` node for it)

```ocaml
(** Identity function **)
let id λx.x in
id 10
```

### Pipe operator
`a | b` is the same as `(b a)`. Very useful to create functional pilelines.
```ocaml
9 | sqrt | + 5 | / 2 | - 1
```

## Conventions
### Point-free style
In built-in functions point-free style is preferred, meaning the "value" argument is always LAST.

This is especially useful in combination with pipes. Consider `/ divisor dividend` and `/ dividend divisor`. The first option is superior because it combines with pipes nicely:
```ocaml
10 | / 5 | - 1
```

### Booleans
We use Church booleans because they are lazy and cool.
```ocaml
let true  \x y.x in
let false \x y.y in
....
```


## Extensions beyond lambda calculus
### Numbers
Church numbers are slow. In order to create a useful program you need fast arithmetic. Arithmetic is handlded by the host language (Rust).

Since the main goal is to have fast counters, we don't need signed numbers or
floats - all Numbers are unsigned integers. If you want signed numbers, floats,
rational, or whatnot - DIY!

### Algebraic Data Types
`#constructor` is a special function that takes `arity` (Number) and gives you an actual data constructor with that arity.

```ocaml
(**Option type **)
let some #constructor 1 in
let none  #constructor 0 in
some 10
```

Constructors are lazy! They merely hold "pointers" to un-evaluated expressions that you passed in. Constructors are values (irreducible).

You can now use `#match` function, which takes the following parameters:
 1. Constructor - N-ary data constructor you want to match against
 2. Transform - N-ary function that would be called with unwrapped constructor arguments if the value matches
 3. Fallback - a function that takes value (**again**) if match did not happen
 4. Value - the actual value we are testing

```ocaml
#match
    some
    (\inner_value.inner_value + 1)
    (\option.option)
    (some 10)
```

This syntax allows you to create something very similar to exhaustive pattern matching, if you combine it with pipes:
```ocaml
let unwrap_option
  EXHAUSTED 
    | #match none ERROR_EMPTY_OPTION
    | #match some (\inner_value.inner_value)
in

unwrap_option 10
```

Which is the same as creating matcher for `some` and passing another `none`
matcher as fallback, which in turn has `EXHAUSTED` as a fallback. `EXHAUSTED`
here is just a free variable, but you can have anything there, e.g error
reporting.

#### Implementation note
All built-in functions are `Data` nodes in disguise. E.g you can think of `+` as a
reducible data constructor (in this case it's also strict - evaluation of both
arguments is needed to advance evaluation).

### Bytes
Bytes is a primitive array of... bytes (u8). Parser would parse any `"quoted string"` as bytes.
Bytes is immutable, so each time you try to modify it, a new `bytes` is created
(although there are last-reference optimizations that would actually "move" the
value and modify it under the hood, you **should not rely on it**).

```ocaml
let data "hello, world" in
let empty #bytes_new 0 in

let data_first 
  data | #bytes_get 0
in

empty | push data_first
```

### IO
Lambo has a built-in IO monad that describes side-effectful actions. From evaluator point of view, IOs is just Data.

The sole purpose of Runtime is to **unwrap** the IO monad that you might constuct. This **unwrap** procedure is where all the fun happens. You can not invoke **unwrap** manually, only Runtime can.

Lambo does not have types, but for a second let's imagine they exist. Runtime gives you the following tools for constructing and operating IO:
 - `#io_pure value` when unwrapped, returns `value` without any side effects
 - `#io_print bytes` when unwrapped, prints the `bytes` and returns it
 - `#io_read` when unwrapped, reads a line from STDIN and returns it as bytes
 - `#io_flatmap transform io` when evaluated, unwraps the `io` and passes the returned value to `transform`
