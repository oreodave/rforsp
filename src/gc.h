/* gc.h: Allocator and Garbage Collection routines
 * Created: 2026-06-18
 * Author: Aryadev Chavali
 * License: See end of file
 * Commentary:

 This is a GC for the Forsp language.  It's essentially an implementation of
 Cheney's algorithm.  This GC works under the assumption that all heap allocated
 objects are formed of exactly two objects i.e. sizeof(T) = 2 * sizeof(obj_t *).
 */

#ifndef GC_H
#define GC_H

#include "common.h"
#include "obj.h"
#include "vec.h"

#define PAGE_INIT_CAPACITY 128

typedef struct
{
  u64 length, capacity;
  obj_t *data[];
} page_t;

/** Allocate a new `page_t` on the heap.
 * Note that by default the capacity of this page is `PAGE_INIT_CAPACITY`.
 */
page_t *page_make();

/** Allocate a pair of objects in a given page, returning a pointer to the
 * allocation.
 * Will return NIL if there is not enough space within the page to allocate that
 * pair.
 */
obj_t **page_alloc(page_t *);

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
  vec_t roots;
#if DEBUG > 1
  size_t collect_hits;
#endif
} gc_t;

void gc_init();
void gc_stop();

/** Push a new root into the GC structure.
 * This root will be updated and evacuated during collection.
 */
void gc_root_push(obj_t **);

/** Pop a root from the GC structure.
 * This removes this pointer-to-object from the purview of the GC collection.
 */
obj_t **gc_root_pop();

/** Allocate a new object pair using the GC.
 * May collect if there is not enough space in the current page.
 */
obj_t **gc_alloc();

/** Perform a collection as per Cheney's algorithm.
 */
void gc_collect();

#endif

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
