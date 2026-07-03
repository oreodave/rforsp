/* state.c: Global interpreter state
 * Created: 2026-06-18
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "state.h"
#include "primitives.h"

/******************************************************************************
 * State Constructor/Destructor                                               *
 ******************************************************************************/

void state_init()
{
  memset(state, 0, sizeof(state));
  state->atom_true  = intern("t", 1);
  state->atom_quote = intern("quote", 5);
  state->atom_push  = intern("push", 4);
  state->atom_pop   = intern("pop", 3);
  vec_init(&state->read_stack, 3);

  cfstack_init();
  gc_init();
  state->stack = make_vec(256);

  state_env_setup();
}

void state_stop()
{
  vec_stop(&state->read_stack);
  for (size_t i = 0; i < state->interned_atoms.length; ++i)
  {
    obj_t *oatom = state->interned_atoms.items[i];
    char *atom   = as_atom(oatom);
    free(atom);
  }
  cfstack_stop();
  vec_stop(&state->interned_atoms);
  gc_stop();
}

/******************************************************************************
 * Stack                                                                      *
 ******************************************************************************/

void push(obj_t *obj)
{
  vec_push(as_vec(state->stack), obj);
}

bool try_pop(obj_t **_out)
{
  vec_t *v = as_vec(state->stack);
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
    obj_t *kv = DIRECT_CAR(env);
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

/// Constant time table for all the primitives we want to setup.

struct PrimRecord
{
  const char *name;
  size_t name_size;
  prim_t *func;
};

#define MAKE_PRIM_RECORD(NAME, FUNC) \
  {.name = (NAME), .name_size = sizeof(NAME) - 1, .func = (FUNC)}

const struct PrimRecord RECORDS[] = {
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
    MAKE_PRIM_RECORD("-", &prim_sub),
    MAKE_PRIM_RECORD("*", &prim_mul),
    MAKE_PRIM_RECORD("nand", &prim_nand),
    MAKE_PRIM_RECORD("<<", &prim_lsh),
    MAKE_PRIM_RECORD(">>", &prim_rsh),
    MAKE_PRIM_RECORD("copy", &prim_copy),
    MAKE_PRIM_RECORD("length", &prim_length),
    MAKE_PRIM_RECORD("vmake", &prim_vmake),
    MAKE_PRIM_RECORD("vpush", &prim_vpush),
    MAKE_PRIM_RECORD("vpop", &prim_vpop),
    MAKE_PRIM_RECORD("vswap", &prim_vswap),
    MAKE_PRIM_RECORD("vget", &prim_vget),
    MAKE_PRIM_RECORD("vset", &prim_vset),
};

void state_env_setup()
{
  obj_t *env = NULL;
  for (size_t i = 0; i < ARRSIZE(RECORDS); ++i)
  {
    const struct PrimRecord *const record = RECORDS + i;
    obj_t *key   = intern(record->name, record->name_size);
    obj_t *value = make_prim(record->func);
    env          = env_define(env, key, value);
  }

  state->env = env;
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
