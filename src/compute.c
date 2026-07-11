/* compute.c: Eval and compute functions implementing CBPV semantics.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "compute.h"
#include "primitives.h"
#include "state.h"

/******************************************************************************
 * Call Frame Cache Helpers                                                   *
 ******************************************************************************/
static inline obj_t *cframe_find(obj_t *key, cframe_t *frame)
{
  size_t least_used = 0;
#if DEBUG & DEBUG_COMPUTE
  printf("compute:cframe_find: Looking for %s\n", as_atom(key));
#endif
  for (size_t i = 0; i < frame->cache.count; ++i)
  {
#if DEBUG & DEBUG_COMPUTE
    printf("\tcomparing to %s {", as_atom(frame->cache.keys[i]));
    print(frame->cache.values[i]);
    printf("}\n");
#endif
    if (frame->cache.keys[i] == key)
    {
#if DEBUG & DEBUG_COMPUTE
      printf("compute:cframe_find: found!\n");
#endif
      // Happy path: we cached the key.
      ++frame->cache.hits[i];
      return frame->cache.values[i];
    }
    else if (frame->cache.hits[i] < frame->cache.hits[least_used])
    {
      least_used = i;
    }
  }

#if DEBUG & DEBUG_COMPUTE
  printf("compute:cframe_find: not found! caching...\n");
#endif
  // Bad path: cache did not store the thing we wanted.
  obj_t *value = env_find(frame->env, key);
  size_t index = frame->cache.count == CFCACHE_CAPACITY ? least_used
                                                        : frame->cache.count++;
  frame->cache.keys[index]   = key;
  frame->cache.values[index] = value;
  frame->cache.hits[index]   = 1;

  return value;
}

/******************************************************************************
 * Call Frame Helpers                                                         *
 ******************************************************************************/

static inline cframe_t cframe_from_clos(obj_t *body, obj_t *env)
{
  return (cframe_t){.body = as_vec(body), .env = env, .ip = 0};
}

static inline obj_t *cframe_pop(cframe_t *cframe)
{
  return cframe->body->items[cframe->ip++];
}

static inline bool cframe_completed(cframe_t *cframe)
{
  return cframe->ip == cframe->body->length;
}

/******************************************************************************
 * Call Frame Stack Helpers                                                   *
 ******************************************************************************/

static cfstack_t *cfstack = &state->cfstack;

static inline bool cfstack_available()
{
  return cfstack->length > 0;
}

static inline void cfstack_pop(void)
{
  --cfstack->length;
}

static inline void cfstack_push(obj_t *comp, obj_t *env)
{
  if (cfstack->capacity - cfstack->length == 0)
  {
    cfstack->capacity *= 2;
    cfstack->frames = realloc(cfstack->frames,
                              cfstack->capacity * sizeof(cfstack->frames[0]));
  }

  STACK_PUSH(cfstack->frames, cfstack->length, cframe_from_clos(comp, env));
}

static inline cframe_t *cfstack_peek(void)
{
  return &STACK_PEEK(cfstack->frames, cfstack->length);
}

/******************************************************************************
 * Compute/Eval                                                               *
 ******************************************************************************/

static inline void call_clos(clos_t *clos, cframe_t *cframe)
{
  if (!cframe_completed(cframe))
    // There is still work to be done in the current cframe.  Thus, we
    // establish a new cframe for this closure.
    cfstack_push(clos->body, clos->env);
  else
    // If there's no work to be done, why inflate the call stack?  Just
    // reuse the current call cframe and continue.
    *cframe = cframe_from_clos(clos->body, clos->env);
}

static inline void call(obj_t *to_call, cframe_t *cframe)
{
  if (IS_CLOS(to_call))
  {
    auto new_clos = as_clos(to_call);
    call_clos(new_clos, cframe);
  }
  else if (IS_REC(to_call))
  {
    auto raw_clos = as_rec(to_call);
    // Push the recursive function itself onto the stack, to be consumed by
    // itself during the call.
    push(to_call);
    call_clos(raw_clos, cframe);
  }
  else if (IS_PRIM(to_call))
  {
    auto callable = TYPED_UNTAG(to_call, prim_t *);
    if (callable == &prim_pop)
      cframe->cache.count = 0;
    callable(&cframe->env);
  }
  else
  {
    push(to_call);
  }
}

static inline void eval_atom(obj_t *cmd, cframe_t *cframe)
{
  if (cmd == state->atom_quote)
  {
    if (cframe_completed(cframe))
      FAIL("Expected data following a quote form");
    push(cframe_pop(cframe));
  }
  else if (cmd == state->atom_if)
  {
    if (cframe->ip > cframe->body->length - 2)
      FAIL("Expected branches following a if form");

    obj_t *t_branch = cframe_pop(cframe);
    obj_t *f_branch = cframe_pop(cframe);
    obj_t *chosen   = pop() == state->atom_true ? t_branch : f_branch;

    assert(IS_VEC(chosen));
    chosen = make_clos(chosen, cframe->env);
    call(chosen, cframe);
  }
  else
  {
    auto val = cframe_find(cmd, cframe);
    call(val, cframe);
  }
}

/** eval function: the basic object-by-object evaluation model.
 * This is called by `compute` (which see) on each member of a closure.
 * eval pushes onto the call frame stack only when a closure is called.
 */
static inline void eval(cframe_t *cframe)
{
  auto cmd = cframe_pop(cframe);

  switch (get_tag(cmd))
  {
  case TAG_ATOM:
  {
    eval_atom(cmd, cframe);
  }
  break;
  case TAG_VEC:
  {
    auto new_clos = make_clos(cmd, cframe->env);
    push(new_clos);
  }
  break;
  case TAG_NIL:
  case TAG_NUM:
  case TAG_CLOS:
  case TAG_REC:
  case TAG_PRIM:
  case TAG_PAIR:
  default:
    push(cmd);
    break;
  }
}

/** Compute function: the main driver of Forsp.

 * This is the core loop for evaluation in Forsp, which iterates through the
 * call frame stack.  The top most call frame is used in `eval` (which see).
 */
void compute(obj_t *comp, obj_t *env)
{
  cfstack_push(comp, env);
  for (cframe_t *cframe = cfstack_peek(); cfstack_available();
       cframe           = cfstack_peek())
  {
    if (cframe_completed(cframe))
    {
      cfstack_pop();
      continue;
    }

#if DEBUG & DEBUG_COMPUTE
    printf("compute:sp[%u]:ip[%lu]: {", cfstack->length, cframe->ip);
    print(cframe->body->items[cframe->ip]);
    printf("} <- ");
    print(TAG_CANON(cframe->body, TAG_VEC));
    printf("\n");
    printf("stack: ");
    print(state->stack);
    printf("\n");
    printf("env: ");
    print(cframe->env);
    printf("\n");
#endif

    eval(cframe);
#if DEBUG & DEBUG_COMPUTE
    BORDER();
#endif
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
