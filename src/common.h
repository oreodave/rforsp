/* common.h: Common types, macros, global state, and function declarations.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#ifndef COMMON_H
#define COMMON_H

#include <assert.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define DEBUG 1

#define FAIL(...)                 \
  do                              \
  {                               \
    fprintf(stderr, "FAIL: ");    \
    fprintf(stderr, __VA_ARGS__); \
    fprintf(stderr, "\n");        \
    abort();                      \
  } while (0)

#define MAX(A, B) ((A) > (B) ? (A) : (B))
#define MIN(A, B) ((A) < (B) ? (A) : (B))

typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;

typedef int8_t i8;
typedef int16_t i16;
typedef int32_t i32;
typedef int64_t i64;

/*******************************************************************
 * Object
 ******************************************************************/

typedef enum Tag
{
  TAG_NIL  = 0,
  TAG_ATOM = 1,
  TAG_NUM  = 2,
  TAG_PAIR = 3,
  TAG_CLOS = 4,
  TAG_PRIM = 5,
} tag_t;

typedef struct obj obj_t;

#define TAG_CANON(X, T)   ((obj_t *)(((u64)(X) << 8) | (T)))
#define TAG_TYPE(X, TYPE) (TAG_CANON(X, TAG_##TYPE))
#define UNTAG(X)          ((u64)(X) >> 8)
#define GET_TAG(X)        ((u64)(X) & 0xFF)

#define IS_NIL(obj)  (GET_TAG(obj) == TAG_NIL)
#define IS_ATOM(obj) (GET_TAG(obj) == TAG_ATOM)
#define IS_NUM(obj)  (GET_TAG(obj) == TAG_NUM)
#define IS_PAIR(obj) (GET_TAG(obj) == TAG_PAIR)
#define IS_CLOS(obj) (GET_TAG(obj) == TAG_CLOS)
#define IS_PRIM(obj) (GET_TAG(obj) == TAG_PRIM)

typedef struct pair
{
  obj_t *car, *cdr;
} pair_t;

typedef struct clos
{
  obj_t *body, *env;
} clos_t;

typedef void(prim_t)(obj_t **);

/** Dynamic array of objects.
 * Classical implementation.

 * TODO: Consider if an inline-header vector may be better.
 */
typedef struct vec
{
  u32 capacity, length;
  obj_t **items;
} vec_t;

// Global State
typedef struct state state_t;
struct state
{
  const char *input_name; // name for source of input data
  char *input_str;        // input data string used by read()
  size_t input_len;       // input data length used by read()
  size_t input_pos;       // input data position used by read()

  vec_t read_stack; // defered obj to emit from read

  vec_t interned_atoms; // interned atoms list
  obj_t *atom_true;     // atom: t
  obj_t *atom_quote;    // atom: quote
  obj_t *atom_push;     // atom: push
  obj_t *atom_pop;      // atom: pop

  obj_t *stack; // top-of-stack (implemented with pairs)
  obj_t *env;   // top-level / initial environment
};

extern state_t state[1];

/*******************************************************************
 * Tagging
 ******************************************************************/

tag_t get_tag(obj_t *ptr);

obj_t *make_atom(const char *str, size_t len);
obj_t *make_num(int64_t num);
obj_t *make_pair(obj_t *car, obj_t *cdr);
obj_t *make_clos(obj_t *body, obj_t *env);
obj_t *make_prim(prim_t *func);

char *as_atom(obj_t *obj);
i64 as_num(obj_t *obj);
pair_t *as_pair(obj_t *obj);
clos_t *as_clos(obj_t *obj);
prim_t *as_prim(obj_t *obj);

/*******************************************************************
 * Basic API
 ******************************************************************/

obj_t *car(obj_t *obj);
obj_t *cdr(obj_t *obj);

void vec_init(vec_t *vec, size_t initial_capacity);
void vec_push(vec_t *vec, obj_t *item);
void vec_push_mult(vec_t *vec, obj_t **items, u64 num_items);
bool vec_try_pop(vec_t *vec, obj_t **ret);

obj_t *intern(const char *atom_buf, size_t atom_len);
bool obj_equal(obj_t *a, obj_t *b);

/*******************************************************************
 * Basic I/O
 ******************************************************************/

obj_t *read(void);
void print(obj_t *obj);

/*******************************************************************
 * Environment
 ******************************************************************/

obj_t *env_find(obj_t *env, obj_t *key);
obj_t *env_define(obj_t *env, obj_t *key, obj_t *val);
obj_t *env_define_prim(obj_t *env, const char *name, void (*func)(obj_t **env));
void state_env_setup();

/*******************************************************************
 * Stack
 ******************************************************************/

void push(obj_t *obj);
bool try_pop(obj_t **_out);
obj_t *pop(void);

/*******************************************************************
 * Eval
 ******************************************************************/

void eval(obj_t *expr, obj_t **env);
void compute(obj_t *comp, obj_t *env);

/*******************************************************************
 * Primitives
 ******************************************************************/

// required
void prim_push(obj_t **env);
void prim_pop(obj_t **env);
void prim_eq(obj_t **_);
void prim_cons(obj_t **_);
void prim_car(obj_t **_);
void prim_cdr(obj_t **_);
void prim_cswap(obj_t **_);
void prim_tag(obj_t **_);
void prim_read(obj_t **_);
void prim_print(obj_t **_);

// helpful
void prim_stack(obj_t **_);
void prim_env(obj_t **env);
void prim_sub(obj_t **_);
void prim_mul(obj_t **_);
void prim_nand(obj_t **_);
void prim_lsh(obj_t **_);
void prim_rsh(obj_t **_);

#endif

/* Copyright (c) 2024 Anthony Bonkoski
 * Copyright (C) 2026 Aryadev Chavali
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.
 *
 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.
 */
