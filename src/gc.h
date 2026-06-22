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

#ifndef GC_CHUNK_SLOTS
#define GC_CHUNK_SLOTS (1LU << 11)
#endif
constexpr size_t GC_CHUNK_DATA_SIZE   = GC_CHUNK_SLOTS << 4;
constexpr size_t GC_CHUNK_MARK_WORDS  = (GC_CHUNK_SLOTS + 63) >> 6;
constexpr size_t GC_THRESHOLD_DEFAULT = GC_CHUNK_DATA_SIZE >> 1;

/** Chunk of memory managed by the GC.
 * `mark_bits`: bitmap for marks across all slots.
 * `live_bits`: bitmap for whether a given slot is live.
 * `data`: raw data where allocations are stored.
 */
typedef struct
{
  u64 mark_bits[GC_CHUNK_MARK_WORDS];
  u64 live_bits[GC_CHUNK_MARK_WORDS];
  u8 data[GC_CHUNK_DATA_SIZE];
} gc_chunk_t;

/** Dynamic array of chunks used for stable growth of memory.
 * `length`: number of chunks currently live.
 * `capacity`: number of chunk pointers available to use.
 * `chunks`: array of chunk pointers.
 */
typedef struct
{
  u64 length, capacity;
  gc_chunk_t **chunks;
} gc_pool_t;

/** GC metadata used during collection.
 * `alloc_live`: number of live allocations.
 * `alloc_bytes`: number of live allocations in bytes.
 * `threshold`: number of bytes when collection should trigger.
 */
typedef struct
{
  size_t alloc_live;
  size_t threshold;
#if DEBUG & DEBUG_GC
  size_t num_collections;
#endif
} gc_metadata_t;

/** General GC data structure.
 * `metadata`: see `gc_metadata_t`.
 * `free_list`: list of "free" i.e. dead allocations.
 * `pool`: see `gc_pool_t`.
 */
typedef struct
{
  gc_metadata_t metadata;

  void **stack_base;

  void *free_list;
  gc_pool_t pool;
} gc_t;

/** Initialise the GC.
 * NOTE: This doesn't allocate anything on the heap.
 */
void gc_init(void **stack_base);

/** Stop the GC and free all associated memory.
 */
void gc_stop(void);

/** Reset the GC.
 * This may be used to initialise the GC, or reset for multiple calls.
 */
void gc_reset(void);

/** Allocate a tagged pair or closure slot.
 * Must pass TAG_PAIR or TAG_CLOS.
 */
__attribute__((noinline)) obj_t *gc_alloc(tag_t tag);

/** Mark an obj_t* as reachable.
 * Call for each root before gc_sweep().
 */
void gc_mark_obj(obj_t *obj);

/** Sweep unmarked slots back into the free list.
 * Returns number freed.
 */
size_t gc_sweep(void);

/** Performs a complete Mark + Sweep cycle.
 * Returns number freed.
 */
size_t gc_collect(void);

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
