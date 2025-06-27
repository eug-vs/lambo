# Lambo - Lambda Calculus Compiler
> This README file is used as an input to the main program. It only sees and
> computes only within code fences ("```")

## Basics
Expression on each line is automatically evaluated.

## Built-ins
The language is designed to be very minimal. It has almost nothing besides
lambda engine, but this is enough to be Turing-Complete.

Built-ins work in Church Encoding, but you are not forced to use it - interpret
the output however you want. Since all built-ins are functions, you can use
them to create your own functions.

### Assert
**After** expression is evaluated, a separate checker makes sure that every
time a variable with name `assert` is called, it's argument is alpha-equivalent
to `λx.λy.x` (TRUE in Church encoding).

The function itself is **not** a keyword, and it does not have any built-in
implementation. This means you can actually use `assert` as parameter name in
your functions - if that parameter is not reduced in the final expression -
assert checker will do it job.

```js
// Equivalent to assert true in other languages
assert λx.λy.x
```

### Beta-equivalence
This operator is only needed because it can't be otherwise derived from within
Lambda Calculus.

It works like this and returns the lambda term corresponding to the Church
boolean (`λx.λy.x` = `true`, `λx.λy.y` = `false`):
```js
assert (#eq foo foo)
assert (#eq (#eq foo bar) λx.λy.y)

// λa.a and λb.b is the same function - just different variable names
assert (#eq λa.a λb.b)
```

## Variable declaration
This is only a syntax sugar. Lambda Calculus does not have named variables, but you can
emulate them via argument names. Writing `let <name> <expr>` simply wraps 
every evaluation below it into a closure providing `<expr>` as a named argument.

Using `eval` (or `assert`) below will be translated into `eval
(λ<variable_name>.<expr_to_eval> <variable_expr>)`.

Example of creating an alias to built-in `#eq` function:
```js
let = #eq
assert (= (= foo foo) λx.λy.x)
```

## Church encoding
Below follows an example of implementating Church encoding in Lambo.

### Boolean logic
```js
let true λx.λy.x
let false λx.λy.y

let not λbool.bool false true
assert (= true (not false))

let and λp.λq.((p q) p)
assert (= (and true x) x)
assert (= (and false x) false)

let if λcondition.λthen.λelse.((condition then) else)
assert (= (if true then else) then)
assert (= (if false then else) else)
```

### Arithmetic
N-th Church number is a function that is essentially "Repeat N times".
```js
// Fun fact: this is actually equivalent to "false"
let 0 λf.λx.x

let succ λn.λf.λx.(f ((n f) x))
let 1 (succ 0)
let 2 (succ 1)

assert (= (0 f x) x)
assert (= (1 f x) (f x))
assert (= (2 f x) (f (f x)))

assert (= (succ (succ 0)) 2)
assert (= (2 succ 0) 2)

// A + B is A, with "succ" function applied B times
let + λa.λb.((b succ) a)

// A * B is (+ A) function applied B times to 0
let * λa.λb.((b (+ a)) 0)
let double (* 2)

// A ^ B is (* A) function applied B times to 1
let ^ λa.λb.((b (* a)) 1)
let square (^ 2)

assert (= (double 2) (square 2))

let 4 (double 2)
let 8 (double 4)
let 16 (double 8)
let 32 (double 16)
let 64 (double 32) // Good luck actually using this value until compiler is optimized

assert (= ((+ ((+ 4) 8)) 4) 16)
assert (= (square 4) 16)
```
