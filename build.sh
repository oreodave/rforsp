#!/bin/bash

CFLAGS="-std=c23 -Wall -Wextra -Wpedantic -Werror -ggdb -Isrc"
LDFLAGS=""

cc $CFLAGS -o forsp src/main.c src/tagging.c src/reader.c src/print.c src/environment.c src/stack.c src/eval.c src/primitives.c $LDFLAGS
