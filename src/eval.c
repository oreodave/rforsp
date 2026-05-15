/* eval.c: Eval and compute functions implementing CBPV semantics.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "common.h"

void eval(obj_t *expr, obj_t **env);

void compute(obj_t *comp, obj_t *env)
{
  if (DEBUG)
  {
    printf("compute: ");
    print(comp);
  }

  while (comp != NULL)
  {
    obj_t *cmd = car(comp);
    comp       = cdr(comp);

    if (cmd == state->atom_quote)
    {
      if (comp == NULL)
        FAIL("Expected data following a quote form");
      push(car(comp));
      comp = cdr(comp);
      continue;
    }

    eval(cmd, &env);
  }
}

void eval(obj_t *expr, obj_t **env)
{
  if (DEBUG)
  {
    printf("eval: ");
    print(expr);
  }

  if (IS_ATOM(expr))
  {
    obj_t *val = env_find(*env, expr);
    if (IS_CLOS(val))
    {
      auto clos = as_clos(val);
      compute(clos->body, clos->env);
    }
    else if (IS_PRIM(val))
    {
      as_prim(val)(env);
    }
    else
    {
      push(val);
    }
  }
  else if (IS_NIL(expr) || IS_PAIR(expr))
  {
    push(make_clos(expr, *env));
  }
  else
  {
    push(expr);
  }
}

/* Copyright (c) 2024 Anthony Bonkoski
 * Copyright (C) 2026 Aryadev Chavali
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.
 *
 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.
 */
