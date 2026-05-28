/* tagging.c: Object creation, tagging, and accessor functions.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "common.h"

state_t state[1];

tag_t get_tag(obj_t *ptr)
{
  return (tag_t)GET_TAG(ptr);
}

obj_t *make_atom(const char *str, size_t len)
{
  char *atom_str = malloc(len + 1);
  memcpy(atom_str, str, len);
  atom_str[len] = '\0';

  return TAG_TYPE(atom_str, ATOM);
}

obj_t *make_num(int64_t num)
{
  assert(num == ((num << 8) >> 8));
  return TAG_TYPE(num, NUM);
}

obj_t *make_pair(obj_t *car, obj_t *cdr)
{
  pair_t *pair = malloc(sizeof(*pair));
  *pair        = (typeof(*pair)){car, cdr};
  return TAG_TYPE(pair, PAIR);
}

obj_t *make_clos(obj_t *body, obj_t *env)
{
  clos_t *clos = malloc(sizeof(*clos));
  *clos        = (typeof(*clos)){body, env};
  return TAG_TYPE(clos, CLOS);
}

obj_t *make_prim(prim_t *func)
{
  return TAG_TYPE(func, PRIM);
}

obj_t *make_vec(u64 init_cap)
{
  vec_t *vec    = malloc(sizeof(*vec));
  vec->capacity = init_cap;
  vec->length   = 0;
  vec->items    = init_cap == 0 ? NULL : malloc(sizeof(obj_t *) * init_cap);
  return TAG_TYPE(vec, VEC);
}

char *as_atom(obj_t *obj)
{
  if (!IS_ATOM(obj))
    return NULL;
  return (char *)UNTAG(obj);
}

i64 as_num(obj_t *obj)
{
  assert(IS_NUM(obj));
  return ((i64)obj) >> 8;
}

pair_t *as_pair(obj_t *obj)
{
  if (!IS_PAIR(obj))
    return NULL;
  return (pair_t *)UNTAG(obj);
}

clos_t *as_clos(obj_t *obj)
{
  if (!IS_CLOS(obj))
    return NULL;
  return (clos_t *)UNTAG(obj);
}

prim_t *as_prim(obj_t *obj)
{
  if (!IS_PRIM(obj))
    return NULL;
  return (prim_t *)UNTAG(obj);
}

vec_t *as_vec(obj_t *obj)
{
  if (!IS_VEC(obj))
    return NULL;
  return (vec_t *)UNTAG(obj);
}

obj_t *car(obj_t *obj)
{
  if (!IS_PAIR(obj))
    return NULL;
  return as_pair(obj)->car;
}

obj_t *cdr(obj_t *obj)
{
  if (!IS_PAIR(obj))
    return NULL;
  return as_pair(obj)->cdr;
}

void vec_push(vec_t *vec, obj_t *item)
{
  if (!vec)
    return;

  if (vec->capacity - vec->length == 0)
  {
    vec->capacity = MAX(vec->length + 1, vec->capacity * 2);
    vec->items    = realloc(vec->items, sizeof(item) * vec->capacity);
  }
  vec->items[vec->length] = item;
  vec->length++;
}

void vec_push_mult(vec_t *vec, obj_t **items, u64 num_items)
{
  if (!vec)
    return;
  if (vec->capacity - vec->length < num_items)
  {
    vec->capacity = MAX(vec->length + num_items, vec->capacity * 2);
    vec->items    = realloc(vec->items, sizeof(*items) * vec->capacity);
  }
  memcpy(vec->items + vec->length, items, sizeof(*items) * num_items);
  vec->length += num_items;
}

bool vec_try_pop(vec_t *vec, obj_t **ret)
{
  if (!vec || !vec->length)
  {
    return false;
  }
  else
  {
    vec->length--;
    *ret = vec->items[vec->length];
    return true;
  }
}

obj_t *intern(const char *atom_buf, size_t atom_len)
{
  for (u64 i = 0; i < state->interned_atoms.length; ++i)
  {
    obj_t *elem     = state->interned_atoms.items[i];
    char *elem_atom = as_atom(elem);
    if (atom_len == strlen(elem_atom) &&
        0 == memcmp(atom_buf, elem_atom, atom_len))
      return elem;
  }

  obj_t *atom = make_atom(atom_buf, atom_len);
  vec_push(&state->interned_atoms, atom);
  return atom;
}

bool obj_equal(obj_t *a, obj_t *b)
{
  return (a == b);
}

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
