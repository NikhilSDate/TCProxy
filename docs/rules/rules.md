# The Rule Language

A simple, s-expression based DSL for specifying proxy rules.
Rules are executed in the order they are specified;
that is, rules will be executed from top to bottom.
If the outcome for the packet is `CONTINUE`, then the rule will continue execution to the next rule.

## Features

- variable declaration
- conditionals
    - querying metadata
    - find and replace content matching (only in `OPAQUE` mode)

Each rule results in one of four possible outcomes:

- `DROP`: silently drop the inbound packet
- `REJECT`: respond with a CONNECTION_REFUSED error
- `(REDIRECT <target> <port>)`: forward the inbound packet to the specified target
- `(REWRITE <find> <replace>)`: rewrite packet content via regex substitution

There is also a special outcome `CONTINUE` which allows for chaining rules.

## Syntax

Our DSL uses a lisp-like syntax.

```bnf
<s_exp>       ::= <atom>
                | <list> .

<list>        ::= "nil"
                | "(" <list_part> ")" .
<list_part>   ::= <s_exp> <list_part>
                | <s_exp> .

<atom>        ::= <ident>
                | <number>
                | <string>
                | <bool> .

<ident>       ::= <letter> <ident_part>
<ident_part>  ::= <empty>
                | <letter> <ident_part>
                | <number> <ident_part>
                | "-" <ident_part>
                | "?" <ident_part>
                | "!" <ident_part>.

<string>      ::= "\"" <string_part> "\"" .
<string_part> ::= <empty>
                | <ascii> <string_part> .

;; <letter>, <number>, <ascii>, <bool> defined elsewhere
```

## Keywords & Built-in Functions

- `(set-mode <mode>)`: Every rule file **must** begin with setting the proxy mode to either `OPAQUE` or `TRANSPARENT`.
- `(def-var <name> <value>)`: Define a variable.
- `(def-rule <name> <body>)`: Define a rule.
- `(if <predicate> <consequent> <alternative>)`: Evaluate the predicate; if `#t`, evaluate the consequent; otherwise,
  evaluate the alternative.
- `DROP`, `REJECT`, `REDIRECT`, `REWRITE`, `CONTINUE` are all reserved for the corresponding outcome.

## Examples

```lisp
;; OPAQUE or TRANSPARENT
;; compiler error if missing or not one of the above
(set-mode OPAQUE)

(def-var bad-ip "192.0.1.2")

(def-rule simple-rewrite
    (if (exact? metadata-source bad-ip)
        (REWRITE "^bar$" "baz")
        CONTINUE))

(def-rule simple-rule
    (if (exact? metadata-source bad-ip)
        DROP
        (REDIRECT "127.0.0.1" 80)))
```
