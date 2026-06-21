CC=cc
CFLAGS=-std=c23 -Wall -Wextra -Wpedantic -Wswitch-enum -Werror -ggdb -Isrc
LDFLAGS=

DIST=bin
OUT=$(DIST)/forsp

CODE=src/vec.c src/obj.c src/primitives.c src/state.c src/compute.c \
		src/reader.c src/print.c src/main.c

HEADERS=src/common.h src/gc.h src/vec.h src/obj.h src/primitives.h src/state.h \
		src/compute.h

EXAMPLES=examples/church-numerals.fp examples/currying.fp examples/demo.fp \
		examples/factorial.fp examples/fibonacci-functional.fp examples/forsp.fp \
		examples/higher-order-functions.fp examples/tutorial.fp

TESTS=$(DIST)/test_gc

.PHONY:
all: $(OUT) $(TESTS)

$(OUT): $(HEADERS) $(CODE) | $(DIST)
	$(CC) $(CFLAGS) -o $@ $(CODE)

$(DIST)/test_gc: tests/libtest.h $(HEADERS) tests/test_gc.c src/gc.c | $(DIST)
	$(CC) $(CFLAGS) -Itests -o $@ tests/test_gc.c src/gc.c

$(DIST):
	mkdir -p $(DIST)

scratch.fp:
	echo "()" > scratch.fp

.PHONY: clean
clean:
	rm -rfv $(DIST)

ARGS=
.PHONY: run
run: $(OUT)
	./$(OUT) $(ARGS)

.PHONY: examples
examples: $(OUT)
	set -e; \
	for example in $(EXAMPLES); do \
		echo "<$$example>"; \
		./$(OUT) $$example; \
	done

.PHONY: tests
tests: $(TESTS)
	set -e; \
	for test in $(TESTS); do \
		./$$test; \
	done

.PHONY: scratch
scratch: $(OUT) scratch.fp
	./$(OUT) scratch.fp
