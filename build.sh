#!/bin/bash

CFLAGS="-std=c23 -Wall -Wextra -Wpedantic -Werror -O2 -g"
LDFLAGS=""

gcc $CFLAGS -o forsp forsp.c $LDFLAGS
