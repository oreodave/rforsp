CC=gcc
CFLAGS=-std=c23 -Wall -Wextra -Wpedantic -Wswitch-enum -Werror -ggdb -O2
LDFLAGS=
DEFS=

DIST=bin
OUT=$(DIST)/rforsp

LIB=src/obj.c src/gc.c src/primitives.c src/cfstack.c src/state.c \
		src/compute.c src/reader.c src/print.c

HEADERS=src/common.h src/obj.h src/gc.h src/primitives.h src/cfstack.h \
		src/state.h src/compute.h

EXAMPLES=examples/church-numerals.rfp examples/currying.rfp examples/demo.rfp \
		examples/factorial.rfp examples/fibonacci-functional.rfp examples/forsp.rfp \
		examples/higher-order-functions.rfp examples/tutorial.rfp \
		examples/bigrange-list.rfp examples/bigrange-vec.rfp \
		examples/loads-of-cons.rfp \
		examples/collatz.rfp \

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

BENCH_EXAMPLE=./examples/forsp.fp

MEMPERF_OUT=$(DIST)/memperf.txt
.PHONY: memperf
memperf: $(OUT)
	valgrind -s \
		--show-leak-kinds=all \
		--leak-check=full \
		--track-origins=yes \
		./$(OUT) $(BENCH_EXAMPLE) \
	&> $(MEMPERF_OUT);
	@cat $(MEMPERF_OUT);

CALLPERF_OUT=$(DIST)/callperf.out
.PHONY: callperf
callperf: $(OUT)
	valgrind --tool=callgrind \
		--callgrind-out-file=$(CALLPERF_OUT) \
		--collect-jumps=yes \
		--dump-instr=yes \
		./$(OUT) $(BENCH_EXAMPLE);

BEFORE=$(DIST)/forsp.original
AFTER=$(OUT)
BENCH_BEFORE=$(BENCH_EXAMPLE)
BENCH_AFTER=$(BENCH_EXAMPLE)

BENCHMARK_OUT=$(DIST)/benchmark.txt
.PHONY: benchmark
benchmark: $(OUT)
	poop -d 10000 \
		"$(BEFORE) $(BENCH_BEFORE)" \
		"$(AFTER) $(BENCH_AFTER)" \
	> $(BENCHMARK_OUT);
	@cat $(BENCHMARK_OUT);

scratch.fp:
	echo "()" > scratch.fp

.PHONY: scratch
scratch: $(OUT) scratch.fp
	./$(OUT) scratch.fp

compile_commands.json:
	bear -- $(MAKE) -B
