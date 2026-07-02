/* obj.h: Object API (pointer tagging and helpful functions)
 * Created: 2026-06-17
 * Author: Aryadev Chavali
 * License: See end of file
 */

#ifndef OBJ_H
#define OBJ_H

#include "common.h"

typedef enum Tag
{
  TAG_NIL  = 0,
  TAG_ATOM = 1,
  TAG_NUM  = 2,
  TAG_PAIR = 3,
  TAG_CLOS = 4,
  TAG_VEC  = 5,
  TAG_PRIM = 6,
} tag_t;

typedef struct obj obj_t;

#define TAG_CANON(X, T)   ((obj_t *)(((uintptr_t)(X) << 8) | (T)))
#define TAG_TYPE(X, TYPE) (TAG_CANON(X, TAG_##TYPE))
#define UNTAG(X)          ((uintptr_t)(X) >> 8)
#define GET_TAG(X)        ((uintptr_t)(X) & 0xFF)

#define IS_NIL(obj)  (GET_TAG(obj) == TAG_NIL)
#define IS_ATOM(obj) (GET_TAG(obj) == TAG_ATOM)
#define IS_NUM(obj)  (GET_TAG(obj) == TAG_NUM)
#define IS_PAIR(obj) (GET_TAG(obj) == TAG_PAIR)
#define IS_CLOS(obj) (GET_TAG(obj) == TAG_CLOS)
#define IS_PRIM(obj) (GET_TAG(obj) == TAG_PRIM)
#define IS_VEC(obj)  (GET_TAG(obj) == TAG_VEC)

#define IS_ALLOC(OBJ) (IS_PAIR(OBJ) || IS_CLOS(OBJ) || IS_VEC(OBJ))

#define DIRECT_UNTAG(X, T) ((T)UNTAG(X))
#define DIRECT_CAR(O)      (((pair_t *)(UNTAG(O)))->car)
#define DIRECT_CDR(O)      (((pair_t *)(UNTAG(O)))->cdr)

typedef struct pair
{
  obj_t *car, *cdr;
} pair_t;

typedef struct clos
{
  obj_t *body, *env;
} clos_t;

typedef struct vec
{
  u32 length, capacity;
  obj_t **items;
} vec_t;

typedef void(prim_t)(obj_t **);

obj_t *make_atom(const char *str, size_t len);
obj_t *make_num(int64_t num);
obj_t *make_pair(obj_t *car, obj_t *cdr);
obj_t *make_clos(obj_t *body, obj_t *env);
obj_t *make_vec(obj_t *body, obj_t *env);
obj_t *make_prim(prim_t *func);

static inline tag_t get_tag(obj_t *ptr)
{
  return (tag_t)GET_TAG(ptr);
}

static inline char *as_atom(obj_t *obj)
{
  if (!IS_ATOM(obj))
    return NULL;
  return (char *)UNTAG(obj);
}

static inline i64 as_num(obj_t *obj)
{
  assert(IS_NUM(obj));
  // NOTE: Arithmetic shift
  return ((i64)obj) >> 8;
}

static inline pair_t *as_pair(obj_t *obj)
{
  if (!IS_PAIR(obj))
    return NULL;
  return DIRECT_UNTAG(obj, pair_t *);
}

static inline clos_t *as_clos(obj_t *obj)
{
  if (!IS_CLOS(obj))
    return NULL;
  return DIRECT_UNTAG(obj, clos_t *);
}

static inline vec_t *as_vec(obj_t *obj)
{
  if (!IS_VEC(obj))
    return NULL;
  return DIRECT_UNTAG(obj, vec_t *);
}

static inline prim_t *as_prim(obj_t *obj)
{
  if (!IS_PRIM(obj))
    return NULL;
  return DIRECT_UNTAG(obj, prim_t *);
}

static inline obj_t *car(obj_t *obj)
{
  auto pair = as_pair(obj);
  return pair ? pair->car : NULL;
}

static inline obj_t *cdr(obj_t *obj)
{
  auto pair = as_pair(obj);
  return pair ? pair->cdr : NULL;
}

static inline bool obj_equal(obj_t *a, obj_t *b)
{
  return (a == b);
}

obj_t *intern(const char *atom_buf, size_t atom_len);

void vec_init(vec_t *vec, size_t initial_capacity);
void vec_stop(vec_t *vec);
void vec_push(vec_t *vec, obj_t *item);
void vec_push_mult(vec_t *vec, obj_t **items, u64 num_items);
bool vec_try_pop(vec_t *vec, obj_t **ret);

typedef struct
{
  tag_t tag;
  union
  {
    char *as_atom;
    i64 as_num;
    pair_t as_pair;
    clos_t as_clos;
    vec_t as_vec;
    prim_t *as_prim;
  };
} obj_canon_t;

obj_canon_t as_canon(obj_t *);

#endif

/* Copyright (c) 2024 Anthony Bonkoski
 * Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
