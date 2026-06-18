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
  TAG_PRIM = 5,
  TAG_FWD  = 6, // NOTE: Used for allocator
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
#define IS_FWD(obj)  (GET_TAG(obj) == TAG_FWD)

typedef struct pair
{
  obj_t *car, *cdr;
} pair_t;

typedef struct clos
{
  obj_t *body, *env;
} clos_t;

typedef void(prim_t)(obj_t **);

tag_t get_tag(obj_t *ptr);

obj_t *make_atom(const char *str, size_t len);
obj_t *make_num(int64_t num);
obj_t *make_pair(obj_t *car, obj_t *cdr);
obj_t *make_clos(obj_t *body, obj_t *env);
obj_t *make_prim(prim_t *func);
obj_t *make_fwd(obj_t *ptr);

char *as_atom(obj_t *obj);
i64 as_num(obj_t *obj);
pair_t *as_pair(obj_t *obj);
clos_t *as_clos(obj_t *obj);
prim_t *as_prim(obj_t *obj);
obj_t *as_fwd(obj_t *obj);

obj_t *car(obj_t *obj);
obj_t *cdr(obj_t *obj);
obj_t *intern(const char *atom_buf, size_t atom_len);
bool obj_equal(obj_t *a, obj_t *b);

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
