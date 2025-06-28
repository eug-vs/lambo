# Lambo - Lambda Calculus Compiler
> This README file is used as an input to the main program. It only sees and
> computes only within code fences ("```")

## Basics
Expression on each line is automatically evaluated.

## Built-in functions
The language is designed to be very minimal. It has almost nothing besides
lambda engine, but this is enough to be Turing-Complete.

### Throw
**#throw** is a special function which, once evaluated, will abort the program
and print the thrown value. The simplest program `#throw x` will abort
immediately :boom:

Usually you would wrap this function in another lambda so that it's not
evaluated eagerly.
```js
// This does not abort until the function is evaluated
λx.#throw x
```

### Beta-equivalence
`#eq` is a special function which tests beta-equivalence of two arguments and
returns result in a form of Church boolean (`λx.λy.x` = `true`, `λx.λy.y` =
`false`).

This operator is only needed because it can't be otherwise derived from within
Lambda Calculus. 

```js
#eq foo foo
// => λx.λy.x
#eq foo bar
// => λx.λy.y

// λa.a and λb.b is the same function - just different variable names
#eq λa.a λb.b
// => λx.λy.x
```
Of course you are not forced to use Church encoding - interpret the output
however you want!

### Variable declaration
This is only a syntax sugar. Lambda Calculus does not have named variables, but you can
emulate them via argument names. Writing `let <name> <expr>` simply wraps 
every evaluation below it into a closure providing `<expr>` as a named argument.

Every expression below will be transformed into `(λ<variable_name>.<original>) <variable_expr>`.

Example of creating an alias to built-in `#eq` function:
```js
let = #eq
```

#### Assert
Let's build our first useful function: `assert_eq`. It will take two input
values and throw an error if they are not equal.

```js
// Define helpers
let assertion_pass λa.λb.PASS
// FAIL, LEFT, RIGHT are just made-up names (free variables).
// As long as they are not reduced to anything, this is very
// useful to convey some meaning of the thrown value
let assertion_fail λa.λb.#throw (FAIL (LEFT a) (RIGHT b))

// (#eq a b) is a Church boolean, therefore works nicely as if/then/else selector
let assert_eq λa.λb.((#eq a b) assertion_pass assertion_fail) a b

assert_eq λx.x ((λy.λz.z) y)
// => PASS

// Shorthand for assert_eq true
let assert (assert_eq λx.λy.x)

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
assert_eq (not false) true

let and λp.λq.((p q) p)
assert_eq (and true x) x
assert_eq (and false x) false

let if λcondition.λthen.λelse.((condition then) else)
assert_eq (if true then else) then
assert_eq (if false then else) else
```

### Arithmetic
N-th Church number is a function that is essentially "Repeat N times".
```js
// Fun fact: this is actually equivalent to "false"
let 0 λf.λx.x

let succ λn.λf.λx.(f ((n f) x))
let 1 (succ 0)
let 2 (succ 1)

assert_eq (0 f x) x
assert_eq (1 f x) (f x)
assert_eq (2 f x) (f (f x))

assert_eq (succ (succ 0)) 2
assert_eq (2 succ 0) 2

// A + B is A, with "succ" function applied B times
let + λa.λb.((b succ) a)

// A * B is (+ A) function applied B times to 0
let * λa.λb.((b (+ a)) 0)
let double (* 2)

// A ^ B is (* A) function applied B times to 1
let ^ λa.λb.((b (* a)) 1)
let square (^ 2)

assert_eq (double 2) (square 2)

let 4 (double 2)
let 8 (double 4)
let 16 (double 8)
let 32 (double 16)
let 64 (double 32) // Good luck actually using this value until compiler is optimized

assert_eq ((+ ((+ 4) 8)) 4) 16
assert_eq (square 4) 16
```
