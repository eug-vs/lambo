
```js
[
#io_putchar (+ 50 54) | #io_flatmap \_.
#io_putchar 101 | #io_flatmap \_.
#io_putchar 108 | #io_flatmap \_.
#io_putchar 108 | #io_flatmap \_.
#io_putchar 111 | #io_flatmap \_.
#io_putchar 10
]


[
with id \x.x in
// Church booleans
with true  (\x.\y.x) in
with false (\x.\y.y) in

// Pairs
with pair #constructor 2 in

// Lists
with nil      #constructor 0 in
with cons     #constructor 2 in

with Y λf.((λx.f (x x)) (λx.f (x x))) in

with println
  Y \println.\str.
    #match cons
        (\h.\t.h | #io_putchar | #io_flatmap (\_.t | println))
        (\_.#io_putchar 10) // newline at the end
    str
in

with map
  Y \map.\f.\l.
      #match cons
        (\h.\t.cons (h | f) (t | map f))
        (\_.nil)
      l
in

with enumerate \l.
    Y (\g.\i.\lst.
        #match cons
            (\h.\t.cons (pair h i) (g (+ i 1) t))
            (\_.nil)
        lst
    ) 0 l
in

with to_upper \char.(=num char 32) 32 (- 32 char) in

with append
  Y \append.\what.\to.
        #match cons
            (\h.\t.cons h (append t to))
            (\_.to)
        what
in

with modulo \modulus.\num.num | - (num | / modulus | * modulus) in
with is_even \num.num | modulo 2 | =num 0 in

with sarcasm \str.str | enumerate | map \p.
    #match pair
        (\char.\index.is_even index char (to_upper char))
        (\_.UNREACHABLE)
    p
in

with repeat
  Y \r.\f.\n.\x.
    (=num n 0)
      x
      (r f (- 1 n) (f x))
in

// "Hello World" as Church-encoded list of numbers
with hello
  (cons (+ 50 54) (cons 101 (cons 108 (cons 108 (cons 111
  (cons 32 (cons 119 (cons 111 (cons 114 (cons 108 (cons 100 nil))))))))))) in

hello | println | #io_flatmap \_.
hello | map to_upper | println | #io_flatmap \_.
with space (cons 32 nil) in 
with repeat_string \times.\str.repeat (append str) (- 1 times) str in
hello | sarcasm | append space | repeat_string 10 | println
]
```
