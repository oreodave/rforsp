/* cfstack.h: Call frame stack for compute/eval
 * Created: 2026-07-03
 * Author: Aryadev Chavali
 * License: See end of file
 */

#ifndef CFSTACK_H
#define CFSTACK_H

#include "obj.h"

typedef struct
{
  vec_t *body;
  obj_t *env;
  u64 ip;
} cframe_t;

#define CFSTACK_DEFAULT_CAPACITY (1 << 7)

typedef struct
{
  u32 length, capacity;
  cframe_t *frames;
} cfstack_t;

void cfstack_init();
void cfstack_stop();

#endif

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
