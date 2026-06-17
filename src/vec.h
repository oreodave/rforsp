/* vec.h: Vectors
 * Created: 2026-06-17
 * Author: Aryadev Chavali
 * License: See end of file
 */

#ifndef VEC_H
#define VEC_H

#include "common.h"
#include "obj.h"

/** Dynamic array of objects.
 * Classical implementation.

 * TODO: Consider if an inline-header vector may be better.
 */
typedef struct vec
{
  u32 length, capacity;
  obj_t **items;
} vec_t;

void vec_init(vec_t *vec, size_t initial_capacity);
void vec_push(vec_t *vec, obj_t *item);
void vec_push_mult(vec_t *vec, obj_t **items, u64 num_items);
bool vec_try_pop(vec_t *vec, obj_t **ret);

#endif

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
