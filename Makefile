CC=cc
CFLAGS=-std=c23 -Wall -Wextra -Wpedantic -Wswitch-enum -Werror -ggdb -Isrc
LDFLAGS=
OUT=forsp

CODE=src/tagging.c src/primitives.c src/stack.c src/environment.c src/eval.c src/reader.c src/print.c src/main.c
EXAMPLES=examples/church-numerals.fp examples/currying.fp examples/demo.fp \
		examples/factorial.fp examples/fibonacci-functional.fp examples/forsp.fp examples/higher-order-functions.fp examples/tutorial.fp

$(OUT): src/common.h $(CODE)
	$(CC) $(CFLAGS) -o $@ $(CODE)

ARGS=
.PHONY: run
run: $(OUT)
	./$(OUT) $(ARGS)

.PHONY: examples
examples: $(OUT)
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
