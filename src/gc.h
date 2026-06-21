/* gc.h: Pool-based mark-sweep garbage collector for pairs and closures.
 * Created: 2026-06-21
 * Author: Aryadev Chavali
 * License: See end of file
 *
 * Manages only TAG_PAIR and TAG_CLOS allocations via a non-moving chunked
 * pool.  Atoms, numbers, NIL, and primitives are not managed.
 *
 * Roots are registered externally (conservative stack scan, global state,
 * or explicit calls to gc_mark_obj).  Mark/sweep are exposed as separate
 * operations so root providers can plug in freely.
 */

#ifndef GC_H
#define GC_H

#include "common.h"
#include "obj.h"

#define GC_CHUNK_SLOTS      4096
#define GC_CHUNK_DATA_SIZE  (GC_CHUNK_SLOTS * 16)
#define GC_CHUNK_MARK_WORDS (GC_CHUNK_SLOTS / 64)

typedef struct gc_chunk
{
  struct gc_chunk *next;
  uint64_t mark_bits[GC_CHUNK_MARK_WORDS];
  uint64_t alloc_bits[GC_CHUNK_MARK_WORDS];
  uint8_t data[GC_CHUNK_DATA_SIZE];
} gc_chunk_t;

void gc_init(void);
void gc_stop(void);

/* Allocate a tagged pair or closure slot.  Must pass TAG_PAIR or TAG_CLOS. */
__attribute__((noinline)) obj_t *gc_alloc(tag_t tag);

/* Mark an obj_t* as reachable.  Call for each root before gc_sweep(). */
void gc_mark_obj(obj_t *obj);

/* Sweep unmarked slots back into the free list.  Returns number freed. */
size_t gc_sweep(void);

/* Convenience: mark + sweep in one call.  gc_mark_obj must have been
 * called for all roots before this.  Returns number freed. */
size_t gc_collect(void);

size_t gc_alloc_count(void);
size_t gc_bytes_allocated(void);

#endif

/* Copyright (c) 2024 Anthony Bonkoski
 * Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
