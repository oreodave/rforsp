/* stack.c: Value stack push/pop operations.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "common.h"

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
