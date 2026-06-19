/* gc.c: Allocator and Garbage Collector implementation
 * Created: 2026-06-19
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "gc.h"

page_t *page_make()
{
  // FIXME: assuming pair-likes are the only allocated type.
  page_t *page =
      calloc(1, sizeof(*page) + (sizeof(*page->data) * PAGE_INIT_CAPACITY * 2));
  page->length   = 0;
  page->capacity = PAGE_INIT_CAPACITY;
  return page;
}

obj_t **page_alloc(page_t *page)
{
  if (page->capacity - page->length == 0)
  {
    return NULL;
  }

  // FIXME: assuming pair-likes are the only allocated type.
  obj_t **items = &page->data[page->length * 2];
  page->length += 2;
  return items;
}

bool page_resize(page_t **page, u64 new_cap)
{
  if (!page || !*page || page[0]->capacity >= new_cap)
    return false;
  // FIXME: assuming pair-likes are the only allocated type.
  page[0] = realloc(page[0],
                    sizeof(*page[0]) + (sizeof(*page[0]->data) * new_cap * 2));
  page[0]->capacity = new_cap;
  return true;
}

void gc_init(gc_t *gc)
{
  gc->backup  = page_make();
  gc->current = page_make();
}

void gc_stop(gc_t *gc)
{
  free(gc->backup);
  free(gc->current);
}

obj_t **as_alloc(obj_t *obj)
{
  return (obj_t **)UNTAG(obj);
}

obj_t **gc_alloc(gc_t *gc)
{
  obj_t **pair = page_alloc(gc->current);
  while (!pair)
  {
    // Collect and try again.
    gc_collect(gc);
    pair = page_alloc(gc->current);

    if (!pair)
    {
      // If allocation has failed following collection, we need to increase the
      // size of our backup page and try again.
      page_resize(&gc->backup, gc->backup->capacity * 2);
    }
  }

  // Ensure the capacity of the backup page is at least the capacity of the
  // current page
  if (gc->backup->capacity < gc->current->capacity)
  {
    page_resize(&gc->backup, gc->current->capacity);
  }

  return pair;
}

/** Check if the given `obj` is allocated in `gc->backup` for the given `gc`.
 * NOTE: If `obj` is not an allocated object, this function returns true
 * vacuously.
 */
bool in_backup_space(gc_t *gc, obj_t *obj)
{
  if (!IS_ALLOC(obj))
    return true;
  uintptr_t ptr          = (uintptr_t)UNTAG(obj);
  uintptr_t backup_start = (uintptr_t)gc->backup->data;
  uintptr_t backup_end   = (uintptr_t)gc->backup->data +
                           (gc->backup->length * sizeof(*gc->backup->data));
  return backup_start <= ptr && backup_end >= ptr;
}

void gc_collect(gc_t *gc)
{
  // TODO: complete algorithm
  (void)gc;
  FAIL("Not done");

  // Swap current page with backup page once evacuation is over.
  // Ensure backup page is completely reset with regards to length.
}

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
