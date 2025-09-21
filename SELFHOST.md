```js
[
with id \x.x in

// Recursion
with Y λf.((λx.f (x x)) (λx.f (x x))) in

// Lists
with nil      #constructor 0 in
with cons     #constructor 2 in

with map
  Y \map.\f.\l.
      #match cons
        (\h.\t.cons (h | f) (t | map f))
        (\_.nil)
      l
in

with reverse_list
    (Y \reverse_list.\acc.\l.
    #match cons
      (\h.\t.reverse_list (cons h acc) t)
      (\_.acc)
    l) nil
in


// IO
with println
  Y \println.\str.
    #match cons
        (\h.\t.h | #io_putchar | #io_flatmap (\_.t | println))
        (\_.#io_putchar 10) // newline at the end
    str
in


with lambda #constructor 1 in
with var #constructor 1 in 
with free_var #constructor 1 in 
with call #constructor 2 in 

with expr_to_str \expr.
    with number_to_char (+ 48) in
    with lambda_char 76 in
    with space_char 32 in
    with open_paren 40 in
    with close_paren 41 in

    (Y \expr_to_str.\expr.\list.expr |
        (EXHAUSTED 
            | #match free_var (\char.list | cons char)
            | #match var (\index.list | cons (number_to_char index))
            | #match lambda (\body.list | cons lambda_char | expr_to_str body)
            | #match call (\func.\param.
                list
                | cons open_paren
                | expr_to_str func
                | cons space_char
                | expr_to_str param
                | cons close_paren
            )
        )
    ) expr nil
in

with substitute
    (Y \substitute.\depth.\wth.\expr.expr |
        (id
            | #match var (\index.(=num index depth) wth expr)
            | #match lambda (\body.body | substitute (depth | + 1) wth | lambda)
            | #match call (\func.\param.call
                (func | substitute depth wth)
                (param | substitute depth wth)
            )
        )

    ) 1
in

with evaluate
    Y \evaluate.\expr.expr | (
        id // Variables are returned without modification
            | #match lambda (\body.body | evaluate | lambda)
            | #match call (\func.\param.
                    func | evaluate | #match lambda
                        (\body.body | evaluate | substitute param) 
                        (\_.call func (evaluate param))
            )
    )
in

with TRUE (var 2) | lambda | lambda in
with FALSE (var 1) | lambda | lambda in
with X free_var 88 in
with Y free_var 89 in

with EXPR call (call TRUE X) Y in
EXPR | expr_to_str | reverse_list | println | #io_flatmap \_.
EXPR | evaluate | expr_to_str | reverse_list | println
]
```
