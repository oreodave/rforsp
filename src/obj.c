/* obj.c: Object API (pointer tagging and helpful functions)
 * Created: 2026-06-18
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "obj.h"
#include "state.h"

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

obj_t *make_fwd(obj_t *ptr)
{
  return TAG_TYPE(ptr, FWD);
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
  // NOTE: Arithmetic shift
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

obj_t *as_fwd(obj_t *obj)
{
  if (!IS_FWD(obj))
    return NULL;
  return (obj_t *)UNTAG(obj);
}

obj_t *car(obj_t *obj)
{
  auto pair = as_pair(obj);
  return pair ? pair->car : NULL;
}

obj_t *cdr(obj_t *obj)
{
  auto pair = as_pair(obj);
  return pair ? pair->cdr : NULL;
}

obj_t *intern(const char *atom_buf, size_t atom_len)
{
  for (u64 i = 0; i < state->interned_atoms.length; ++i)
  {
    auto elem      = state->interned_atoms.items[i];
    auto elem_atom = as_atom(elem);
    if (atom_len == strlen(elem_atom) &&
        0 == memcmp(atom_buf, elem_atom, atom_len))
      return elem;
  }

  auto atom = make_atom(atom_buf, atom_len);
  vec_push(&state->interned_atoms, atom);
  return atom;
}

bool obj_equal(obj_t *a, obj_t *b)
{
  return (a == b);
}

/* Copyright (c) 2024 Anthony Bonkoski
 * Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
