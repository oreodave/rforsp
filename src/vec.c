/* vec.c: Vector implementation
 * Created: 2026-06-18
 * Author: Aryadev Chavali
 * License: See end of file
 * Commentary:
 */

#include "obj.h"

void vec_init(vec_t *vec, size_t initial_capacity)
{
  if (!vec || vec->capacity >= initial_capacity)
    return;
  vec->capacity = initial_capacity;
  vec->items    = calloc(initial_capacity, sizeof(*vec->items));
}

void vec_stop(vec_t *vec)
{
  free(vec->items);
  memset(vec, 0, sizeof(*vec));
}

void vec_push(vec_t *vec, obj_t *item)
{
  if (!vec)
    return;

  if (vec->capacity - vec->length == 0)
  {
    vec->capacity = MAX(vec->length + 1, vec->capacity * 2);
    vec->items    = realloc(vec->items, sizeof(item) * vec->capacity);
  }
  vec->items[vec->length] = item;
  vec->length++;
}

void vec_push_mult(vec_t *vec, obj_t **items, u64 num_items)
{
  if (!vec)
    return;
  if (vec->capacity - vec->length < num_items)
  {
    vec->capacity = MAX(vec->length + num_items, vec->capacity * 2);
    vec->items    = realloc(vec->items, sizeof(*items) * vec->capacity);
  }
  memcpy(vec->items + vec->length, items, sizeof(*items) * num_items);
  vec->length += num_items;
}

bool vec_try_pop(vec_t *vec, obj_t **ret)
{
  if (!vec || !vec->length)
  {
    return false;
  }
  else
  {
    vec->length--;
    *ret = vec->items[vec->length];
    return true;
  }
}

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
