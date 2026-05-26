CC=cc
CFLAGS=-std=c23 -Wall -Wextra -Wpedantic -Werror -ggdb -Isrc
LDFLAGS=
OUT=forsp

CODE=$(shell find src/ -name "*.c")
EXAMPLES=examples/church-numerals.fp examples/currying.fp examples/demo.fp \
		examples/factorial.fp examples/fibonacci-functional.fp examples/forsp.fp examples/higher-order-functions.fp examples/tutorial.fp

$(OUT): $(CODE)
	$(CC) $(CFLAGS) -o $@ $^

ARGS=""
.PHONY: run
run: $(OUT)
	./$^

.PHONY: examples
examples: $(OUT)
	for example in $(EXAMPLES); do \
		echo "<$$example>"; \
		./$^ $$example; \
	done

scratch.fp:
	echo "()" > scratch.fp

.PHONY: scratch
scratch: $(OUT) scratch.fp
	./$^

.PHONY: clean
clean:
	rm -fv $(OUT)
