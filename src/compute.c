/* compute.c: Eval and compute functions implementing CBPV semantics.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "compute.h"
#include "state.h"

void compute(obj_t *comp, obj_t *env)
{
#if DEBUG & DEBUG_COMPUTE
  printf("compute: ");
  print(comp);
  printf("\n");
#endif

  while (comp != NULL)
  {
    auto cmd = car(comp);
    comp     = cdr(comp);

    switch (get_tag(cmd))
    {
    case TAG_ATOM:
      // quote is the one special operator.
      if (cmd == state->atom_quote)
      {
        if (comp == NULL)
          FAIL("Expected data following a quote form");
        push(car(comp));
        comp = cdr(comp);
        continue;
      }

      // Otherwise perform a lookup and "call" the value.
      auto val = env_find(env, cmd);
      if (IS_CLOS(val))
      {
        auto clos = as_clos(val);
        compute(clos->body, clos->env);
      }
      else if (IS_PRIM(val))
      {
        as_prim(val)(&env);
      }
      else
      {
        push(val);
      }
      break;
    case TAG_NIL:
    case TAG_PAIR:
      push(make_clos(cmd, env));
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
