# Forsp: A Forth+Lisp Hybrid Lambda Calculus Language

Forsp is a hybrid language combining Forth and Lisp.

Forsp is a minimalist language, requiring only 10 primitive functions
and a relatively small amount of code to self-implement the initial
`compute`-`eval` loop.

Forsp has:
  - An S-Expression syntax like Lisp
  - Function abstraction like Lisp
  - Function application like Forth
  - An environment structure like Lisp
  - Lexically-scoped closures like Lisp (Scheme)
  - Cons-cells / lists / atoms like Lisp
  - A value/operand stack like Forth
  - An ability to express the Lambda Calculus
  - A Call-By-Push-Value evaluation order
  - Only 3 syntax special forms: ' ^ $
  - Only 1 eval-time special form: quote


This is a fork of the original sample code developed by `xorvoid`
(Anthony Bonkoski) which you can find
[here](https://github.com/xorvoid/forsp).  It's incredibly minimal
and, thus, a tiny read.  I also highly recommend reading the [original
blog post](https://xorvoid.com/forsp.html) as it does an amazing job
explaining and motivating core decision decisions.

The driving principle behind this fork is redesigning the interpreter,
introducing common optimisations, as well as fleshing out the language
to provide a less bare-bones initial experience.

## Tutorial

The best way to learn Forsp is to [read the
tutorial](examples/tutorial.fp) along with the other available
[examples](examples/).

## Requirements
- C compiler that can compile C23 (tested: gcc 16.1.1 20260430 clang
  22.1.5)
- Make (tested: GNU Make, bmake)

## Building and Demo

Building:
```
make
```

NOTE: Tested on:
- Mac M1
- x86-64 Linux with gcc 16.1.1

Running:
```
make run ARGS=<...>
```
... where `<...>` is a filename.

Testing examples:
```
make examples
```
