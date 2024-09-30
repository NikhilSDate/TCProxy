# The Rule Language

A simple, s-expression based DSL for specifying proxy rules.

## Features

- variable declaration
- conditionals
    - querying metadata
    - regex matching / rewriting on content? (i.e. MOD_REWRITE)

Each rule results in one of four possible outcomes:

- DROP: silently drop the inbound packet
- REJECT: respond with a CONNECTION_REFUSED error
- REDIRECT: forward the inbound packet to the specified redirect-address
- REWRITE: rewrite packet content via regex substitution

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

TODO

## Examples

```lisp
;; OPAQUE or TRANSPARENT
;; compiler error if missing or not one of the above
(set-mode OPAQUE)

;; variables, perhaps i should rename to def-global?
(def-var bad-ip "192.0.1.2")

(def-rule simple-rule (:target "127.0.0.1" ;; defines local, built-in parameters for rules
                       :port   "80")
    (if (exact? metadata-source bad-ip)    ;; i.e. if metadata-source is an exact match for bad-ip, return #t
        DROP
        REDIRECT))

;; should give a compilation error if config-type != OPAQUE
(def-rule simple-rewrite (:content "foo")
    (if (exact? metadata-source bad-ip)
        (REWRITE "^bar$" "baz") ;; REWRITE does a regex match for arg1 and replaces it with arg2
        (simple-rule)))
        ;; one question is how do we want to handle composition of rules?
        ;; should all the rules run together (in which case this should get rid of simple-rule and replace it with something else,
        ;; and also decide on the order of rule execution, what to do with diverging responses from rules [precedence for operations? e.g. DROP > REJECT > REDIRECT?])
        ;; or should we just have a (def-pipeline rule1 rule2 rule3 ...) that is used to determine the order?
```
