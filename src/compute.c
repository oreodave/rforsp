/* compute.c: Eval and compute functions implementing CBPV semantics.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "compute.h"
#include "state.h"

/******************************************************************************
 * Frame Stack Helpers                                                        *
 ******************************************************************************/

static inline bool fstack_available()
{
  return state->fstack.length > 0;
}

static inline void fstack_push(obj_t *comp, obj_t *env)
{
  if (state->fstack.capacity - state->fstack.length == 0)
  {
    state->fstack.capacity *= 2;
    state->fstack.frames =
        realloc(state->fstack.frames,
                state->fstack.capacity * sizeof(state->fstack.frames[0]));
  }

  state->fstack.frames[state->fstack.length++] =
      (clos_t){.body = comp, .env = env};
}

static inline clos_t *fstack_peek(void)
{
  return &state->fstack.frames[state->fstack.length - 1];
}

static inline void fstack_pop(void)
{
  --state->fstack.length;
}

/******************************************************************************
 * Compute/Eval                                                               *
 ******************************************************************************/

/** eval function: the basic object-by-object evaluation model.
 * This is called by `compute` (which see) on each member of a closure.
 * eval pushes onto the call frame stack only when a closure is called.
 */
static inline void eval(clos_t *frame)
{
  auto cmd    = DIRECT_CAR(frame->body);
  frame->body = DIRECT_CDR(frame->body);

  switch (get_tag(cmd))
  {
  case TAG_ATOM:
    // quote is the one special operator.
    if (cmd == state->atom_quote)
    {
      if (frame->body == NULL)
        FAIL("Expected data following a quote form");
      push(DIRECT_CAR(frame->body));
      frame->body = DIRECT_CDR(frame->body);
      return;
    }

    // Otherwise perform a lookup and "call" the value.
    auto val = env_find(frame->env, cmd);
    if (IS_CLOS(val))
    {
      auto new_clos = as_clos(val);
      if (frame->body)
        // There is still work to be done in the current frame, establish a new
        // call frame for this closure.
        fstack_push(new_clos->body, new_clos->env);
      else
        // If the current frames work is already complete, we can store this new
        // closure onto it.  This is essentially a `tail call`.
        *frame = *new_clos;
    }
    else if (IS_PRIM(val))
    {
      as_prim(val)(&frame->env);
    }
    else
    {
      push(val);
    }
    break;
  case TAG_NIL:
  case TAG_PAIR:
  {
    auto new_clos = make_clos(cmd, frame->env);
    push(new_clos);
  }
  break;
  case TAG_NUM:
  case TAG_CLOS:
  case TAG_PRIM:
  case TAG_VEC:
  default:
    push(cmd);
    break;
  }
}

/** Compute function: the main driver of Forsp.

 * This is the core loop for evaluation in Forsp.  We keep evaluating a `frame`,
 * member by member through `eval` (which see),
 */
void compute(obj_t *comp, obj_t *env)
{
  fstack_push(comp, env);
  for (clos_t *frame = fstack_peek(); fstack_available(); frame = fstack_peek())
  {
    if (!frame->body)
    {
      fstack_pop();
      continue;
    }

#if DEBUG & DEBUG_COMPUTE
    printf("compute[%ld]: ", state->fstack.length);
    print(frame->body);
    printf("\n");
    printf("stack: ");
    print(state->stack);
    printf("\n");
    printf("env: ");
    print(frame->env);
    printf("\n");
    BORDER();
#endif

    eval(frame);
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
