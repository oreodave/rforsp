CC=gcc
CFLAGS=-std=c23 -Wall -Wextra -Wpedantic -Wswitch-enum -Werror -ggdb -O2
LDFLAGS=
DEFS=

DIST=bin
OUT=$(DIST)/forsp

LIB=src/obj.c src/gc.c src/primitives.c src/cfstack.c src/state.c \
		src/optimiser.c src/compute.c src/reader.c src/print.c

HEADERS=src/common.h src/obj.h src/gc.h src/primitives.h src/cfstack.h \
		src/state.h src/compute.h

EXAMPLES=examples/church-numerals.fp examples/currying.fp examples/demo.fp \
		examples/factorial.fp examples/fibonacci-functional.fp examples/forsp.fp \
		examples/higher-order-functions.fp examples/tutorial.fp \
		examples/bigrange-list.fp examples/bigrange-vec.fp \
		examples/loads-of-cons.fp

$(OUT): $(DIST) $(HEADERS) $(LIB) src/main.c
	$(CC) $(CFLAGS) -Isrc -o $@ $(LIB) src/main.c $(LDFLAGS) $(DEFS)

$(DIST):
	mkdir -p $(DIST)

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

BENCH_EXAMPLE=./examples/forsp.fp

.PHONY: callperf
callperf: $(OUT)
	valgrind --tool=callgrind \
		--callgrind-out-file=$(DIST)/callgrind.out \
		--collect-jumps=yes \
		--dump-instr=yes \
		./$(OUT) $(BENCH_EXAMPLE);

BEFORE=$(DIST)/forsp.original
AFTER=$(OUT)
BENCH_BEFORE=$(BENCH_EXAMPLE)
BENCH_AFTER=$(BENCH_EXAMPLE)

.PHONY: benchmark
benchmark: $(OUT)
	poop -d 10000 \
		"$(BEFORE) $(BENCH_BEFORE)" \
		"$(AFTER) $(BENCH_AFTER)" \
	> $(DIST)/benchmark.txt;
	@cat $(DIST)/benchmark.txt;

scratch.fp:
	echo "()" > scratch.fp

.PHONY: scratch
scratch: $(OUT) scratch.fp
	./$(OUT) scratch.fp

compile_commands.json:
	bear -- $(MAKE) -B
