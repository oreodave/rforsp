/* state.c: Global interpreter state
 * Created: 2026-06-18
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include <stdbit.h>

#include "primitives.h"
#include "state.h"

/******************************************************************************
 * Stack                                                                      *
 ******************************************************************************/

void push(obj_t *obj)
{
  vec_push(as_vec(state->stack), obj);
}

bool try_pop(obj_t **_out)
{
  auto v = as_vec(state->stack);
  return vec_try_pop(v, _out);
}

obj_t *pop(void)
{
  obj_t *ret = NULL;
  if (!try_pop(&ret))
    FAIL("Value Stack Underflow");
  return ret;
}

/******************************************************************************
 * Environment                                                                *
 ******************************************************************************/

obj_t *env_find(obj_t *env, obj_t *key)
{
  if (!IS_ATOM(key))
    FAIL("Expected 'key' (%lx) to be an atom in env_find()", (uintptr_t)key);

  while (IS_PAIR(env))
  {
    auto kv = DIRECT_CAR(env);
    if (key == DIRECT_CAR(kv))
    {
      return DIRECT_CDR(kv);
    }
    env = DIRECT_CDR(env);
  }

  FAIL("Failed to find in key='%s' in environment", as_atom(key));
}

obj_t *env_define(obj_t *env, obj_t *key, obj_t *val)
{
  return make_pair(make_pair(key, val), env);
}

/******************************************************************************
 * State Constructor/Destructor                                               *
 ******************************************************************************/

void state_stop()
{
  vec_stop(&state->read_stack);
  for (size_t i = 0; i < state->interned_atoms.length; ++i)
  {
    auto oatom = state->interned_atoms.items[i];
    auto atom  = as_atom(oatom);
    free(atom);
  }
  cfstack_stop();
  vec_stop(&state->interned_atoms);
  gc_stop();
}

struct PrimRecord
{
  const char *name;
  size_t name_size;
  prim_t *func;
};

struct CachedAtom
{
  const char *name;
  size_t name_size;
  obj_t **place;
};

#define MAKE_CACHED_ATOM(NAME, PLACE) \
  {.name = (NAME), .name_size = sizeof(NAME) - 1, .place = (PLACE)}

const struct CachedAtom CACHED_ATOMS[] = {
    MAKE_CACHED_ATOM("t", &state->atom_true),
    MAKE_CACHED_ATOM("if", &state->atom_if),
    MAKE_CACHED_ATOM("quote", &state->atom_quote),
    MAKE_CACHED_ATOM("push", &state->atom_push),
    MAKE_CACHED_ATOM("pop", &state->atom_pop),
};

#define MAKE_PRIM_RECORD(NAME, FUNC) \
  {.name = (NAME), .name_size = sizeof(NAME) - 1, .func = (FUNC)}

const struct PrimRecord PRIMITIVES[] = {
    MAKE_PRIM_RECORD("push", &prim_push),
    MAKE_PRIM_RECORD("pop", &prim_pop),
    MAKE_PRIM_RECORD("cons", &prim_cons),
    MAKE_PRIM_RECORD("car", &prim_car),
    MAKE_PRIM_RECORD("cdr", &prim_cdr),
    MAKE_PRIM_RECORD("eq", &prim_eq),
    MAKE_PRIM_RECORD("cswap", &prim_cswap),
    MAKE_PRIM_RECORD("tag", &prim_tag),
    MAKE_PRIM_RECORD("read", &prim_read),
    MAKE_PRIM_RECORD("print", &prim_print),
    MAKE_PRIM_RECORD("stack", &prim_stack),
    MAKE_PRIM_RECORD("env", &prim_env),
    MAKE_PRIM_RECORD("+", &prim_add),
    MAKE_PRIM_RECORD("-", &prim_sub),
    MAKE_PRIM_RECORD("*", &prim_mul),
    MAKE_PRIM_RECORD("/", &prim_div),
    MAKE_PRIM_RECORD("&", &prim_bitwise_and),
    MAKE_PRIM_RECORD("|", &prim_bitwise_or),
    MAKE_PRIM_RECORD("nand", &prim_bitwise_nand),
    MAKE_PRIM_RECORD("<<", &prim_lsh),
    MAKE_PRIM_RECORD(">>", &prim_rsh),
    MAKE_PRIM_RECORD("rec", &prim_rec),
    MAKE_PRIM_RECORD("copy", &prim_copy),
    MAKE_PRIM_RECORD("length", &prim_length),
    MAKE_PRIM_RECORD("vmake", &prim_vmake),
    MAKE_PRIM_RECORD("vpush", &prim_vpush),
    MAKE_PRIM_RECORD("vpop", &prim_vpop),
    MAKE_PRIM_RECORD("vswap", &prim_vswap),
    MAKE_PRIM_RECORD("vget", &prim_vget),
    MAKE_PRIM_RECORD("vset", &prim_vset),
};

void state_init()
{
  memset(state, 0, sizeof(state));
  vec_init(&state->read_stack, 3);
  cfstack_init();

  size_t total_required_atoms =
      stdc_bit_ceil(ARRSIZE(CACHED_ATOMS) + ARRSIZE(PRIMITIVES));
  vec_init(&state->interned_atoms, total_required_atoms);

  for (size_t i = 0; i < ARRSIZE(CACHED_ATOMS); ++i)
  {
    const struct CachedAtom *const catom = CACHED_ATOMS + i;
    auto atom     = intern(catom->name, catom->name_size);
    *catom->place = atom;
  }

  gc_init();
  state->stack = make_vec(256);
  for (size_t i = 0; i < ARRSIZE(PRIMITIVES); ++i)
  {
    const struct PrimRecord *const record = PRIMITIVES + i;
    auto key   = intern(record->name, record->name_size);
    auto value = make_prim(record->func);
    state->env = env_define(state->env, key, value);
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
