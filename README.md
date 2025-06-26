# Lambo - Lambda Calculus Compiler
> This README file is used as an input to the main program. It only sees and
> computes only within code fences ("```")

## Built-ins
The language is designed to be very minimal. It has almost nothing besides
lambda engine, but this is enough to be Turing-Complete.

### Beta-equivalence
This is the **only** built-in in function: `#eq`. And it is only needed because
it can't be otherwise derived from within Lambda Calculus.

It works like this and returns the lambda term corresponding to the Church
boolean:
```js
assert ((#eq foo) foo) λx.λy.x
assert ((#eq foo) bar) λx.λy.y

// λa.a and λb.b is the same function - just different variable names
assert ((#eq λa.a) λb.b) λx.λy.x
```
Interpret the output however you want, you are not forced to use Church encoding.

`assert <left_expr> <right_expr>` here works as a test for beta-equivalence.

## Variable declaration
This is only a syntax sugar. Lambda Calculus does not have named variables, but you can
emulate them via argument names. Writing `let <name> <expr>` simply wraps 
every evaluation below it into a closure providing `<expr>` as a named argument.

Using `eval` (or `assert`) below will be translated into `eval
(λ<variable_name>.<expr_to_eval> <variable_expr>)`.

Example of creating an alias to built-in `#eq` function:
```js
let = #eq
assert ((= foo) foo) λx.λy.x
assert ((= foo) bar) λx.λy.y
```

## Church encoding
Below follows an example of implementating Church encoding in Lambo.

### Boolean logic
```js
let true λx.λy.x
let false λx.λy.y

assert ((= true) false) false
assert ((= true) true) true

let and λp.λq.((p q) p)

assert ((and true) x) x
assert ((and false) x) false
```
