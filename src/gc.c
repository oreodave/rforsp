/* gc.c: Allocator and Garbage Collector implementation
 * Created: 2026-06-19
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "gc.h"
#include "state.h"

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
  page->length += 1;
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

void gc_init()
{
  state->gc.backup  = page_make();
  state->gc.current = page_make();
  vec_init(&state->gc.roots, 8);
  gc_root_push(&state->stack);
  gc_root_push(&state->env);
}

void gc_stop()
{
  free(state->gc.backup);
  free(state->gc.current);
  vec_stop(&state->gc.roots);
}

void gc_root_push(obj_t **x)
{
  // NOTE: This is kinda illegal, but it's okay cos we're treating all gc.roots
  // as pointers to `obj_t *`.
  vec_push(&state->gc.roots, (obj_t *)x);
}

obj_t **gc_root_pop()
{
  obj_t *ret = NULL;
  if (vec_try_pop(&state->gc.roots, &ret))
  {
    return (obj_t **)ret;
  }
  else
  {
    return NULL;
  }
}

obj_t **as_alloc(obj_t *obj)
{
  return (obj_t **)UNTAG(obj);
}

obj_t **gc_alloc()
{
  obj_t **pair = page_alloc(state->gc.current);
  while (!pair)
  {
    // If allocation has failed, first increase the size of our backup page.
    page_resize(&state->gc.backup, state->gc.backup->capacity * 2);

    // Then, collect and try another allocation.
    gc_collect();
    pair = page_alloc(state->gc.current);

    // if (!pair)
    // {
    // }
  }

  // Ensure the capacity of the backup page is at least the capacity of the
  // current page
  if (state->gc.backup->capacity < state->gc.current->capacity)
  {
    page_resize(&state->gc.backup, state->gc.current->capacity);
  }

  return pair;
}

/** Check if the given `obj` is allocated in `gc->backup` for the given `gc`.
 * NOTE: If `obj` is an unmanaged object, this function returns true vacuously.
 */
bool in_backup_space(obj_t *obj)
{
  if (!IS_ALLOC(obj))
    return true;
  auto ptr          = (uintptr_t)UNTAG(obj);
  auto backup_start = (uintptr_t)state->gc.backup->data;
  // FIXME: assuming pair-likes are the only allocated type.
  auto backup_end =
      (uintptr_t)state->gc.backup->data +
      (state->gc.backup->length * 2 * sizeof(*state->gc.backup->data));
  return backup_start <= ptr && backup_end > ptr;
}

/** Evacuate the given object into backup space, returning the copied object in
 * backup space.
 *
 * NOTE: for non-managed types, this function is the identity.
 */
obj_t *evacuate(obj_t *obj)
{
  if (in_backup_space(obj))
  {
    return obj;
  }

  // Previous branch involving `in_backup_space` takes care of non-allocated
  // objects and those already in backup space.  So we just need to consider
  // allocated objects not in backup space.
  obj_t **items = as_alloc(obj);
  tag_t tag_obj = get_tag(obj);

  if (get_tag(items[0]) == TAG_FWD)
  {
    // If the first item is a FWD object, then this object has been evacuated
    // successfully already.  Follow the link and return the object in backup
    // space.

    // Firstly, we need to recover the copied pointer in backup space from the
    // forwarding pointer.
    void *backup_ptr = as_fwd(items[0]);
    // Then, we need to tag it with the proper type of the initial object.
    obj_t *recovered = TAG_CANON(backup_ptr, tag_obj);
    return recovered;
  }

  // Otherwise, we need to copy this over into backup space and forward the
  // current pointer to this new allocation.
  obj_t **alloc = page_alloc(state->gc.backup);
  assert(alloc && "This allocation should never fail.");

  // FIXME: assuming pair-likes are the only allocated type.
  memcpy(alloc, items, sizeof(*items) * 2);
  items[0]       = TAG_TYPE(alloc, FWD);
  obj_t *new_obj = TAG_CANON(alloc, tag_obj);

  return new_obj;
}

void gc_collect()
{
#if DEBUG > 1
  printf("gc_collect: hit!\n");
  ++state->gc.collect_hits;
#endif
  // 1) Evacuate roots.
  for (size_t i = 0; i < state->gc.roots.length; ++i)
  {
    auto root = (obj_t **)state->gc.roots.items[i];
    *root     = evacuate(*root);
  }

  // 2) Scan backup space to evacuate references.
  for (size_t i = 0; i < state->gc.backup->length; ++i)
  {
    // FIXME: assuming pair-likes are the only allocated type.
    auto pair = &state->gc.backup->data[i * 2];
    pair[0]   = evacuate(pair[0]);
    pair[1]   = evacuate(pair[1]);
  }

  // 3) Swap backup and current pages for future allocations.
  page_t *temp      = state->gc.backup;
  state->gc.backup  = state->gc.current;
  state->gc.current = temp;

  // 4) Reset backup page
  state->gc.backup->length = 0;
}

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
