/* gc.c: Allocator and Garbage Collector implementation
 * Created: 2026-06-19
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "gc.h"

page_t *page_make()
{
  page_t *page =
      calloc(1, sizeof(*page) + (sizeof(*page->data) * PAGE_INIT_CAPACITY));
  page->length   = 0;
  page->capacity = PAGE_INIT_CAPACITY;
  return page;
}

pair_t *page_alloc(page_t *page, pair_t pair)
{
  if (page->capacity - page->length == 0)
  {
    return NULL;
  }
  pair_t *ptr = &page->data[page->length++];
  *ptr        = pair;
  return ptr;
}

bool page_resize(page_t **page, u64 new_cap)
{
  if (!page || !*page || page[0]->capacity >= new_cap)
    return false;
  page[0] =
      realloc(page[0], sizeof(*page[0]) + (sizeof(*page[0]->data) * new_cap));
  return true;
}

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
