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
  gc_reset();
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
  vec_stop(&state->interned_atoms);
}

/******************************************************************************
 * Stack                                                                      *
 ******************************************************************************/

void push(obj_t *obj)
{
  state->stack = make_pair(obj, state->stack);
}

bool try_pop(obj_t **_out)
{
  if (!state->stack)
  {
    return false;
  }

  *_out        = car(state->stack);
  state->stack = cdr(state->stack);
  return true;
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

  for (auto kv = car(env); env != NULL; env = cdr(env), kv = car(env))
    if (key == car(kv))
      return cdr(kv);

  FAIL("Failed to find in key='%s' in environment", as_atom(key));
}

obj_t *env_define(obj_t *env, obj_t *key, obj_t *val)
{
  return make_pair(make_pair(key, val), env);
}

obj_t *env_define_prim(obj_t *env, const char *name, void (*func)(obj_t **env))
{
  return env_define(env, intern(name, strlen(name)), make_prim(func));
}

void state_env_setup()
{
  obj_t *env = NULL;
  env        = env_define_prim(env, "push", &prim_push);
  env        = env_define_prim(env, "pop", &prim_pop);
  env        = env_define_prim(env, "cons", &prim_cons);
  env        = env_define_prim(env, "car", &prim_car);
  env        = env_define_prim(env, "cdr", &prim_cdr);
  env        = env_define_prim(env, "eq", &prim_eq);
  env        = env_define_prim(env, "cswap", &prim_cswap);
  env        = env_define_prim(env, "tag", &prim_tag);
  env        = env_define_prim(env, "read", &prim_read);
  env        = env_define_prim(env, "print", &prim_print);
  env        = env_define_prim(env, "stack", &prim_stack);
  env        = env_define_prim(env, "env", &prim_env);
  env        = env_define_prim(env, "-", &prim_sub);
  env        = env_define_prim(env, "*", &prim_mul);
  env        = env_define_prim(env, "nand", &prim_nand);
  env        = env_define_prim(env, "<<", &prim_lsh);
  env        = env_define_prim(env, ">>", &prim_rsh);

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
