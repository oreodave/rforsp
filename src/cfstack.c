/* cfstack.c: Call frame stack constructor/destructor implementation
 * Created: 2026-07-03
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "cfstack.h"
#include "state.h"

void cfstack_init()
{
  state->cfstack.capacity = CFSTACK_DEFAULT_CAPACITY;
  state->cfstack.length   = 0;
  state->cfstack.frames =
      calloc(state->cfstack.capacity, sizeof(state->cfstack.frames[0]));
}

void cfstack_stop()
{
  free(state->cfstack.frames);
}

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
