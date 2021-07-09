# Script-rs

This project is an exploration of creating a compiler and VM for a dynamic scripting language in Rust.

The language itself is quite simple (no first order functions, no objects) that compiles to an "op-code" language (define as a Rust `enum`) that is a little inspired by [p-code] machines.

Nevertheless, it has:

* A [`lexer`] that convert an input string into a `Vec<Token>`
* A [`parser`] that receives the vector of tokens and return `Ast` ([Abstract Syntax Tree]) nodes
* A [`compiler`] that gets a list of `Ast` and build a sequence of `Op` codes;
* A [`vm`] that interprets the sequence of opcodes.
* Other anciliary modules and functionality, such as error reporting

[p-code]: https://en.wikipedia.org/wiki/P-code_machine
[Abstract Syntax Tree]: https://en.wikipedia.org/wiki/Abstract_syntax_tree
[`lexer`]: src/lexer.rs
[`parser`]: src/parser.rs
[`compiler`]: src/compiler.rs
[`vm`]: src/vm.rs

## The Language

The language is very simple, currently only the following is supported:

* Only tree types: integers, strings and lists
  * Only decimal literal integers (positive only), stored internally as `i64` (it is possible to get negative by using `0-n`)
  * Literal strings allow some escape codes ("\t", "\n", "\r", ...)
  * Literal lists are in the format `[ first_value, second_value ]`
* Variables with the usual possibility of characters (0-9, a-z, '_', '$')
* `while` loops
* `if` statements (but not expressions)

> TODO: a more detailed guide, with list of built-in functions and operators

The language itself is not of great importance for me, in this project, as the idea was to study the design and implementation of compiler + VM, not the language itself.  Features were added more based on how to implement them, than on usefulness.


## The Compiler

The compiler benefits from the simplicity of the LL(1) language.

The **Lexer** converts source (read as `&str`) into a `Vec<Token>`, each `Token` has
* The token type
* The source code range (in char offsets) from where it was read
* The string value of the token itself

*All* input source is processed generating a `Result<Vec<Token>>` with either the complete list of tokens, or an error.  The errors have enough context to indicate where in the source the problem was found.

The **Parser** receives this `Vec<Token>` and returns, incrementally, a `Result<Ast>`.  Note that it does not generate a large AST of the entire source, but returns incrementally from each top-level construction.

Each `Ast` is fed into the **Compiler** that creates code internally, immediatelly, from the receive ASTs.  As a final step the code is processed from optimizations (currently very simple, mostly as placeholder) and to do what usually is done by a linker (checking jump address targets).

The P-code used is defined in [`src/opcodes.rs`](src/opcodes.rs) and very limited (but enough for useful computations).

## The Virtual Machine

The sequence of `Op` codes from the compiler is fed into the **VM** that is a very simple stack-based machine, with separate call and value stacks, a garbage-collected heap and a side mapping of globals from strings.

One of the topics I wanted to explore was how to do the garbage collection in entirely safe Rust for a simple project as this, without having to deal with the darkest corners of making the borrow-checker happy for a situation where performance was not critial (as this is representative of other situations in more usual applications).

The solution that I used is that:
* The *heap* is a `Vec<Option<Value>>` where `Value` is an `enum` with the possible value types (integer, string, list);
* Values are not referenced by their actual value (or reference in the Rust heap), but by a `HeapPtr` that is a thin wrapper around the `usize` index inside heap;
* This means that *all* values are boxed, even integers.  This is a potential major performance problem, but not an issue I care with in this experiment.

With this setup, doing a GC is:
* Create a `Vec<bool>` with the same size as heap
* Iterate over stack and globals, recursing in case of lists, marking accessible values
* Set all non-marked entries into heap to `None`, to indicate free slots

Doing this in Rust, specially the stack manipulations, led that some of the idioms I was used in previous similar projects in C, C++ or GC'ed languages (Java, Ocaml) did not work, as the borrow checker (correctly) refused.  For example modifying the stack, while references to elements inside the stack where held.

Usually the changes were small, re-ordering of accesses, but it was interesting nevertheless.
