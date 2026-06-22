/* compute.c: Eval and compute functions implementing CBPV semantics.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "compute.h"
#include "state.h"

typedef struct
{
  obj_t *comp;
  obj_t *env;
} frame_t;

constexpr size_t COMPUTE_LIMIT = 1024;
static frame_t frames[COMPUTE_LIMIT];

void compute(obj_t *comp, obj_t *env)
{
  memset(frames, 0, sizeof(frames));
  frames[0] = (frame_t){.comp = comp, .env = env};
  i64 depth = 1;

  while (depth > 0)
  {
    frame_t *frame = &frames[depth - 1];
    if (!frame->comp)
    {
      --depth;
      continue;
    }

#if DEBUG & DEBUG_COMPUTE
    printf("compute[%ld]: ", depth);
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

    gc_mark_obj(frame->comp);
    gc_mark_obj(frame->env);
    auto cmd    = car(frame->comp);
    frame->comp = cdr(frame->comp);

    switch (get_tag(cmd))
    {
    case TAG_ATOM:
      // quote is the one special operator.
      if (cmd == state->atom_quote)
      {
        if (frame->comp == NULL)
          FAIL("Expected data following a quote form");
        push(car(frame->comp));
        frame->comp = cdr(frame->comp);
        continue;
      }

      // Otherwise perform a lookup and "call" the value.
      auto val = env_find(frame->env, cmd);
      if (IS_CLOS(val))
      {
        auto new_clos = as_clos(val);
        frames[depth++] =
            (frame_t){.comp = new_clos->body, .env = new_clos->env};
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
      gc_mark_obj(frame->env);
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
