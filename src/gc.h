/* gc.h: Allocator and Garbage Collection routines
 * Created: 2026-06-18
 * Author: Aryadev Chavali
 * License: See end of file
 * Commentary:

 This is a GC for the Forsp language.  It's essentially an implementation of
 Cheney's algorithm.  This GC works under the assumption that all heap allocated
 objects are formed of exactly two objects i.e. sizeof(T) = 2 . sizeof(obj_t *).
 */

#ifndef GC_H
#define GC_H

#include "common.h"
#include "obj.h"
#include "vec.h"

#define PAGE_INIT_CAPACITY 1024

typedef struct
{
  u64 length, capacity;
  pair_t data[];
} page_t;

/** Allocate a new `page_t` on the heap.
 * Note that by default the capacity of this page is `PAGE_INIT_CAPACITY`.
 */
page_t *page_make();

/** Allocate a pair in a given page, returning a pointer to the allocation.
 * Will return NIL if there is not enough space within the page to allocate that
 * pair.
 */
pair_t *page_alloc(page_t *);

/** Resize a page to the given new capacity.
 * Note that this is destructive as the new page may not have the same addresses
 * as the old one.
 * This is due to the nature of `realloc` (which see `man 3 realloc`).
 */
bool page_resize(page_t **, u64);

/** General structure of the GC.
 */
typedef struct
{
  page_t *current, *backup;
} gc_t;

void gc_init(gc_t *);
void gc_stop(gc_t *);

/** Perform a collection as per Cheney's algorithm in the current GC structure.
 */
void gc_collect();

/** Allocate a new object pair in the current GC structure.
 * May collect if there is not enough space in the current page.
 */
pair_t *gc_alloc(gc_t *);

#endif

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
