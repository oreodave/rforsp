/* compute.c: Eval and compute functions implementing CBPV semantics.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "compute.h"
#include "state.h"

static inline void frames_push(obj_t *comp, obj_t *env)
{
  state->frames[state->frame_depth++] = (clos_t){.body = comp, .env = env};
}

static inline clos_t *frames_peek(void)
{
  return &state->frames[state->frame_depth - 1];
}

static inline void eval(clos_t *frame)
{
  auto cmd    = car(frame->body);
  frame->body = cdr(frame->body);

  switch (get_tag(cmd))
  {
  case TAG_ATOM:
    // quote is the one special operator.
    if (cmd == state->atom_quote)
    {
      if (frame->body == NULL)
        FAIL("Expected data following a quote form");
      push(car(frame->body));
      frame->body = cdr(frame->body);
      return;
    }

    // Otherwise perform a lookup and "call" the value.
    auto val = env_find(frame->env, cmd);
    if (IS_CLOS(val))
    {
      auto new_clos = as_clos(val);
      // equivalent to a recursive "compute" call.
      if (frame->body == NULL)
      {
        // TCO for free?????
        *frame = *new_clos;
      }
      else
      {
        frames_push(new_clos->body, new_clos->env);
      }
    }
    else if (IS_PRIM(val))
    {
      prim_t *prim = as_prim(val);
      prim(&frame->env);
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
  default:
    push(cmd);
    break;
  }
}

/** Compute function: the main driver of Forsp.
 */
void compute(obj_t *comp, obj_t *env)
{
  frames_push(comp, env);
  while (state->frame_depth > 0)
  {
    clos_t *frame = frames_peek();
    if (!frame->body)
    {
      --state->frame_depth;
      continue;
    }

#if DEBUG & DEBUG_COMPUTE
    printf("compute[%ld]: ", state->frame_depth);
    print(frame->comp);
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
