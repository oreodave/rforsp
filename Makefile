CC=cc
CFLAGS=-std=c23 -Wall -Wextra -Wpedantic -Wswitch-enum -Werror -ggdb -O1
LDFLAGS=
DEPS=

DIST=bin
OUT=$(DIST)/forsp

LIB=src/vec.c src/obj.c src/gc.c src/primitives.c src/state.c src/compute.c \
		src/reader.c src/print.c

HEADERS=src/common.h src/gc.h src/vec.h src/obj.h src/primitives.h src/state.h \
		src/compute.h

EXAMPLES=examples/church-numerals.fp examples/currying.fp examples/demo.fp \
		examples/factorial.fp examples/fibonacci-functional.fp examples/forsp.fp \
		examples/higher-order-functions.fp examples/tutorial.fp

TESTS=$(DIST)/test_gc

$(OUT): $(HEADERS) $(LIB) src/main.c | $(DIST)
	$(CC) $(DEPS) $(CFLAGS) -Isrc -o $@ $(LIB) src/main.c

$(DIST)/test_gc: $(HEADERS) tests/libtest.h $(LIB) tests/test_gc.c | $(DIST)
	$(CC) $(CFLAGS) -Itests -o $@ tests/test_gc.c $(LIB)

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

.PHONY: memperf
memperf: $(OUT)
	valgrind -s --show-leak-kinds=all --leak-check=full ./$(OUT) ./examples/forsp.fp &> gc.results;

.PHONY: callperf
callperf: $(OUT)
	valgrind --tool=callgrind --callgrind-out-file=bin/callgrind.out ./$(OUT) ./examples/forsp.fp;

.PHONY: scratch
scratch: $(OUT) scratch.fp
	./$(OUT) scratch.fp
