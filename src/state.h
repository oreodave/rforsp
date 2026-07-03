/* state.h: Global interpreter state
 * Created: 2026-06-17
 * Author: Aryadev Chavali
 * License: See end of file
 * Commentary:
 */

#ifndef STATE_H
#define STATE_H

#include "cfstack.h"
#include "common.h"
#include "gc.h"
#include "obj.h"

typedef struct state
{
  char *input_name; // name for source of input data
  char *input_str;  // input data string used by read()
  size_t input_len; // input data length used by read()
  size_t input_pos; // input data position used by read()
  vec_t read_stack; // defered obj to emit from read

  vec_t interned_atoms; // interned atoms list
  obj_t *atom_true;     // atom: t
  obj_t *atom_quote;    // atom: quote
  obj_t *atom_push;     // atom: push
  obj_t *atom_pop;      // atom: pop

  obj_t *stack; // top-of-stack (implemented as a tagged vector)
  obj_t *env;   // top-level / initial environment

  gc_t gc; // allocator for pairs/closures
  // self-managed dynamic array of call frames - used in compute.c
  cfstack_t cfstack;
} state_t;

extern state_t state[1];

/******************************************************************************
 * State Constructor/Destructor                                               *
 ******************************************************************************/
void state_init();
void state_stop();

/******************************************************************************
 * Stack                                                                      *
 ******************************************************************************/

void push(obj_t *obj);
bool try_pop(obj_t **_out);
obj_t *pop(void);

/******************************************************************************
 * Environment                                                                *
 ******************************************************************************/

obj_t *env_find(obj_t *env, obj_t *key);
obj_t *env_define(obj_t *env, obj_t *key, obj_t *val);
obj_t *env_define_prim(obj_t *env, const char *name, void (*func)(obj_t **env));
void state_env_setup();

/******************************************************************************
 * Basic I/O                                                                  *
 ******************************************************************************/

obj_t *read(void);
void print(obj_t *obj);

#endif

/* Copyright (c) 2024 Anthony Bonkoski
 * Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.
 *
 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
