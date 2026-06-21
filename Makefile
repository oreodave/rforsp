CC=cc
CFLAGS=-std=c23 -Wall -Wextra -Wpedantic -Wswitch-enum -Werror -ggdb -Isrc
LDFLAGS=
OUT=forsp

CODE=src/vec.c src/obj.c src/primitives.c src/state.c src/compute.c src/reader.c src/print.c src/main.c
HEADERS=src/common.h src/vec.h src/obj.h src/primitives.h src/state.h src/compute.h
EXAMPLES=examples/church-numerals.fp examples/currying.fp examples/demo.fp \
		examples/factorial.fp examples/fibonacci-functional.fp examples/forsp.fp examples/higher-order-functions.fp examples/tutorial.fp


$(OUT): $(HEADERS) $(CODE)
	$(CC) $(CFLAGS) -o $@ $(CODE)

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

scratch.fp:
	echo "()" > scratch.fp

.PHONY: scratch
scratch: $(OUT) scratch.fp
	./$(OUT) scratch.fp

.PHONY: clean
clean:
	rm -fv $(OUT)
