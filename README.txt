 ___________________________________
/\                                  \
\_|       ______                    |
  |      |  ____|                   |
  |  _ __| |__ ___  _ __ ___ _ __   |
  | | '__|  __/ _ \| '__/ __| '_ \  |
  | | |  | | | (_) | |  \__ \ |_) | |
  | |_|  |_|  \___/|_|  |___/ .__/  |
  |                         | |     |
  |                         |_|     |
  |   ______________________________|_
   \_/________________________________/


rForsp is a hybrid language of Forth and Lisp.  It is based on the
work of `xorvoid` (Anthony Bonkoski) on Forsp, a language with
arguably greater simplicity than both Lisp or Forth on their own.  An
excerpt from the README of the original repo describes it best:

> Forsp has:
> - An S-Expression syntax like Lisp
> - Function abstraction like Lisp
> - Function application like Forth
> - An environment structure like Lisp
> - Lexically-scoped closures like Lisp (Scheme)
> - Cons-cells / lists / atoms like Lisp
> - A value/operand stack like Forth
> - An ability to express the Lambda Calculus
> - A Call-By-Push-Value evaluation order
> - Only 3 syntax special forms: ' ^ $
> - Only 1 eval-time special form: quote

rForsp was forked from the repository `xorvoid/forsp` [1].  For an
exploration into the language itself, there is no better article than
the original blog post [2].  Both the original repository as well as
the article are extremely small reads and magnificently simple to
understand, so I highly recommend looking at them before reading
further here.

----------------------------------------------------------------------
Raison d'être
----------------------------------------------------------------------
rForsp is an exploration into whether Forsp can be more than a
minimalist toy language.  I think there is something really special
here, and I would like to put the work in to see if it can actually be
made into something more capable.

To this end, I hope to introduce language changes and interpreter
optimisations to make the language more competitive against others.
In a sense I am refining the core Forsp language - hence the name
rForsp for a Refined Forsp.

----------------------------------------------------------------------
Requirements
----------------------------------------------------------------------
- A C compiler that supports C23 (gcc and clang have been tested).
- Make (GNU Make and bmake have been tested).

----------------------------------------------------------------------
Building
----------------------------------------------------------------------
Simply run `make` in the root of the project and the project will be
built.  The binary will be located at `./bin/forsp`.

To enable debug logs, such as from the `compute` phase or the GC,
please see `./src/common.h` which describes the log flag system.  Also
see `./Makefile` for more information.

----------------------------------------------------------------------
Running
----------------------------------------------------------------------
`make examples` runs the interpreter against all examples.

`make run ARGS=/path/to/file.fp` will build the interpreter and run it
against a file of your choice.

`./bin/forsp /path/to/file.fp` also works.

----------------------------------------------------------------------
Links
----------------------------------------------------------------------
[1] - https://github.com/xorvoid/forsp
[2] - https://xorvoid.com/forsp.html
