/* compute.c: Eval and compute functions implementing CBPV semantics.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "compute.h"
#include "state.h"

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

static inline void eval_call(obj_t *to_call, cframe_t *cframe)
{
  if (IS_CLOS(to_call))
  {
    auto new_clos = as_clos(to_call);

    if (!cframe_completed(cframe))
      // There is still work to be done in the current cframe.  Thus, we
      // establish a new cframe for this closure.
      cfstack_push(new_clos->body, new_clos->env);
    else
      // If there's no work to be done, why inflate the call stack?  Just
      // reuse the current call cframe and continue.
      *cframe = cframe_from_clos(new_clos->body, new_clos->env);
  }
  else if (IS_PRIM(to_call))
  {
    as_prim(to_call)(&cframe->env);
  }
  else
  {
    push(to_call);
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
    // quote is the one special operator.
    if (cmd == state->atom_quote)
    {
      if (cframe_completed(cframe))
        FAIL("Expected data following a quote form");
      push(cframe_pop(cframe));
      return;
    }

    // Otherwise perform a lookup and "call" the value.
    auto val = env_find(cframe->env, cmd);
    eval_call(val, cframe);
    break;
  case TAG_NIL:
  case TAG_VEC:
  {
    auto new_clos = make_clos(cmd, cframe->env);
    push(new_clos);
  }
  break;
  case TAG_CALL:
    // TODO: Replace when testing payload based TAG_CALL
    eval_call(pop(), cframe);
    break;
  case TAG_NUM:
  case TAG_CLOS:
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
    printf("compute[%ld]: ", cfstack->length);
    print(TAG_CANON(cframe->body, TAG_VEC));
    printf("\n");
    printf("stack: ");
    print(state->stack);
    printf("\n");
    printf("env: ");
    print(cframe->env);
    printf("\n");
    BORDER();
#endif

    eval(cframe);
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
