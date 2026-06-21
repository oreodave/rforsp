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
  pair->car    = car;
  pair->cdr    = cdr;
  return TAG_TYPE(pair, PAIR);
}

obj_t *make_clos(obj_t *body, obj_t *env)
{
  clos_t *clos = malloc(sizeof(*clos));
  clos->body   = body;
  clos->env    = env;
  return TAG_TYPE(clos, CLOS);
}

obj_t *make_prim(prim_t *func)
{
  return TAG_TYPE(func, PRIM);
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

obj_canon_t as_canon(obj_t *obj)
{
  tag_t tag = get_tag(obj);
  switch (tag)
  {
  case TAG_NIL:
    return (obj_canon_t){.tag = tag};
    break;
  case TAG_ATOM:
    return (obj_canon_t){.tag = tag, .as_atom = as_atom(obj)};
    break;
  case TAG_NUM:
    return (obj_canon_t){.tag = tag, .as_num = as_num(obj)};
    break;
  case TAG_PAIR:
    return (obj_canon_t){.tag = tag, .as_pair = *as_pair(obj)};
    break;
  case TAG_CLOS:
    return (obj_canon_t){.tag = tag, .as_clos = *as_clos(obj)};
    break;
  case TAG_PRIM:
    return (obj_canon_t){.tag = tag, .as_prim = as_prim(obj)};
    break;
  default:
    return (obj_canon_t){0};
    break;
  }
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
