/* gc.c: Pool-based mark-sweep garbage collector for pairs and closures.
 * Created: 2026-06-21
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "gc.h"
#include "state.h"

#include <stdbit.h>

static gc_t *gc = &state->gc;

/******************************************************************************
 * Bitmap Helpers                                                             *
 ******************************************************************************/

static inline void bitmap_set(u64 *bits, size_t idx)
{
  bits[idx / 64] |= (1ULL << (idx % 64));
}

static inline bool bitmap_test(const u64 *bits, size_t idx)
{
  return (bits[idx / 64] >> (idx % 64)) & 1;
}

/******************************************************************************
 * Free list Helpers                                                          *
 ******************************************************************************/

/** Push a new slot onto the free list.
 * This slot is presumed to be unused, thus the data owned by it is mutated to
 * become a `gc_free_slot_t`.
 * `chunk_id` and `slot_id` are stored in this type for allocation purposes.
 */
static inline void gc_free_list_push(void *slot, u32 chunk_id, u32 slot_id)
{
  gc_free_slot_t *fslot = slot;
  fslot->next_slot      = gc->free_list;
  fslot->chunk_id       = chunk_id;
  fslot->slot_id        = slot_id;
  gc->free_list         = slot;
}

/** Pop a free slot from the free list.
 */
static inline gc_free_slot_t *gc_free_list_pop()
{
  gc_free_slot_t *free_slot = gc->free_list;
  if (free_slot)
    gc->free_list = free_slot->next_slot;
  return free_slot;
}

/******************************************************************************
 * Slot/Chunk interaction helpers                                             *
 ******************************************************************************/

/** Check if `raw_ptr` is an allocation from `chunk`.
 */
static inline bool gc_ptr_in_chunk(gc_chunk_t *chunk, void *raw_ptr)
{
  auto start = chunk->data;
  auto end   = start + GC_CHUNK_DATA_SIZE;
  return (u8 *)raw_ptr >= start && (u8 *)raw_ptr < end;
}

/** Get the slot that `raw_ptr` belongs to within `chunk`, presuming `raw_ptr`
 * has been verified to have been allocated within `chunk`.
 */
static inline size_t gc_ptr_slot_in_chunk(gc_chunk_t *chunk, void *raw_ptr)
{
  return ((u8 *)raw_ptr - chunk->data) / sizeof(gc_free_slot_t);
}

/******************************************************************************
 * GC Methods                                                                 *
 ******************************************************************************/

void gc_init()
{
  memset(&state->gc, 0, sizeof(state->gc));
  gc->metadata.threshold = GC_THRESHOLD_DEFAULT;
}

void gc_stop()
{
  for (size_t i = 0; i < gc->pool.length; ++i)
  {
    free(gc->pool.chunks[i]);
  }
  free(gc->pool.chunks);
  memset(&state->gc, 0, sizeof(state->gc));
}

void gc_reset()
{
  gc_stop();
  gc_init();
}

/** Construct a new chunk and push it onto the pool.
 */
static inline gc_chunk_t *gc_new_chunk(void)
{
  gc_chunk_t *c = aligned_alloc(16, sizeof(gc_chunk_t));
  if (!c)
  {
    FAIL("GC: failed to allocate chunk");
  }
  memset(c->mark_bits, 0, sizeof(c->mark_bits));
  memset(c->live_bits, 0, sizeof(c->live_bits));

  // Chain all new slots into the free list
  for (size_t i = 0; i < GC_CHUNK_SLOTS; ++i)
  {
    void *slot = c->data + i * 16;
    gc_free_list_push(slot, gc->pool.length, i);
  }

  // Push onto the chunk array in the pool.
  if (!gc->pool.capacity)
  {
    gc->pool.capacity = 1;
    gc->pool.chunks   = malloc(sizeof(*gc->pool.chunks));
  }
  else if (gc->pool.capacity - gc->pool.length == 0)
  {
    gc->pool.capacity *= 2;
    gc->pool.chunks =
        realloc(gc->pool.chunks, sizeof(*gc->pool.chunks) * gc->pool.capacity);
  }

  if (!gc->pool.chunks)
  {
    FAIL("GC: failed to reallocate pool of chunks");
  }

  gc->pool.chunks[gc->pool.length++] = c;

  return c;
}

/** Locate which chunk owns a raw pointer, returning it.
 * If no chunk is found, return NULL.

 * The slot position of the pointer in the chunk is stored in `slot_id`.
 */
static inline gc_chunk_t *gc_find_chunk(void *raw_ptr, size_t *slot_id)
{
  for (size_t i = 0; i < gc->pool.length; ++i)
  {
    auto c = gc->pool.chunks[i];
    if (gc_ptr_in_chunk(c, raw_ptr))
    {
      *slot_id = gc_ptr_slot_in_chunk(c, raw_ptr);
      return c;
    }
  }
  return NULL;
}

static inline bool gc_threshold_met(void)
{
  return gc->enable && gc->metadata.slots_live >= gc->metadata.threshold;
}

__attribute__((noinline)) obj_t *gc_alloc(tag_t tag)
{
  if (gc_threshold_met())
  {
    gc_collect();
  }
  if (!gc->free_list)
  {
#if DEBUG & DEBUG_GC
    printf("GC:alloc: New chunk - no freelist\n");
#endif
    gc_new_chunk();
  }

  gc_free_slot_t *slot = gc_free_list_pop();

#if DEBUG & DEBUG_GC
  // Ensure liveness invariant is met - only done in debug builds.
  assert(
      !bitmap_test(gc->pool.chunks[slot->chunk_id]->live_bits, slot->slot_id));
#endif
  gc->metadata.slots_live++;

  gc_chunk_t *c = gc->pool.chunks[slot->chunk_id];
  bitmap_set(c->live_bits, slot->slot_id);
  if (tag == TAG_VEC)
  {
    bitmap_set(c->vec_bits, slot->slot_id);
  }
  memset(slot, 0, sizeof(*slot));

  return TAG_CANON(slot, tag);
}

void gc_mark_obj(obj_t *obj)
{
  if (!IS_ALLOC(obj))
    return;

  constexpr size_t MARK_STACK_SIZE = 256;
  obj_t *mark_stack[MARK_STACK_SIZE];
  u64 mark_sp           = 0;
  mark_stack[mark_sp++] = obj;

  while (mark_sp > 0)
  {
    obj_t *item = mark_stack[--mark_sp];

    void *raw     = (void *)UNTAG(item);
    size_t idx    = 0;
    gc_chunk_t *c = gc_find_chunk(raw, &idx);

    if (!c || bitmap_test(c->mark_bits, idx))
      continue;

    bitmap_set(c->mark_bits, idx);

    // pair_t and clos_t both start with two obj_t* fields
    obj_t **fields = (obj_t **)raw;
    for (size_t i = 0; i < 2; ++i)
    {
      if (!IS_ALLOC(fields[i]))
        continue;
      else if (mark_sp == MARK_STACK_SIZE - 1)
        gc_mark_obj(fields[i]);
      else
        mark_stack[mark_sp++] = fields[i];
    }
  }
}

size_t gc_sweep(void)
{
#if DEBUG & DEBUG_GC
  printf("GC:sweep: starting...\n");
#endif
  // We must iterate through every chunk and look for unmarked live allocations
  // to eat up into our free list.
  size_t freed = 0;
  for (size_t i = 0; i < gc->pool.length; ++i)
  {
    gc_chunk_t *c = gc->pool.chunks[i];
    for (size_t w = 0; w < GC_CHUNK_MARK_WORDS; ++w)
    {
      // We only want to free those that are both live and unmarked.
      u64 to_free = c->live_bits[w] & ~c->mark_bits[w];
      size_t base = w * 64;

#if DEBUG & DEBUG_GC
      if (to_free)
      {
        printf("\t%lu@%lu...%lu => %d slots to free.\n", i, w * 64,
               (w + 1) * 64, stdc_count_ones(to_free));
      }
#endif

      for (u64 todo = to_free; todo; todo &= todo - 1)
      {
        // Find the lowest bit which is nonzero through a single hardware inst.
        int bit           = stdc_trailing_zeros_ull(todo);
        size_t slot_index = base + bit;

        // Put the slot designated by the bit into the free list.
        void *slot = c->data + slot_index * 16;
        gc_free_list_push(slot, i, slot_index);
      }

      // Clear all live bits in one go.
      freed += stdc_count_ones(to_free);
      c->live_bits[w] &= ~to_free;
    }
    memset(c->mark_bits, 0, sizeof(c->mark_bits));
  }

  gc->metadata.slots_live -= freed;
  gc->metadata.threshold =
      MAX(GC_THRESHOLD_DEFAULT, gc->metadata.slots_live * 2);

#if DEBUG & DEBUG_GC
  printf("GC:sweep: slots_live: %lu\n", gc->metadata.slots_live);
  printf("GC:sweep: threshold: %lu\n", gc->metadata.threshold);
  printf("GC:sweep: freed %lu slots\n", freed);
#endif
  return freed;
}

/** Perform a march through the machine stack, marking objects.
 * This march is done up from the current `rsp` by GC_STACK_MARCH_LIMIT words.
 * Each word is checked to see if it is a valid object that may have been
 * allocated.
 */
static inline void gc_mark_stack_march(void)
{
  constexpr size_t GC_STACK_MARCH_LIMIT = 512;
  __builtin_unwind_init();
  void *sp;
  __asm__ volatile("mov %%rsp, %0" : "=r"(sp));
  void **end = (void **)((u8 *)sp + GC_STACK_MARCH_LIMIT);

#if DEBUG & DEBUG_GC
  printf("GC:collect:stack_march: Iterating from start=%p -> end=%p\n", sp,
         (void *)end);
#endif

  size_t _ = 0;
  for (void **p = sp; p < end; ++p)
  {
    obj_t *maybe = *(obj_t **)p;
    if (IS_ALLOC(maybe))
    {
      void *raw = (void *)UNTAG(maybe);
      if (gc_find_chunk(raw, &_))
      {
#if DEBUG & DEBUG_GC
        printf("GC:collect:stack_march: Marking allocation %p => %p\n",
               (void *)p, raw);
#endif
        gc_mark_obj(maybe);
      }
    }
  }
}

size_t gc_collect(void)
{
#if DEBUG & DEBUG_GC
  ++gc->metadata.num_collections;
  printf("GC:collect: Triggered as %lu live slots vs %lu threshold.\n",
         gc->metadata.slots_live, gc->metadata.threshold);
#endif

  gc_mark_stack_march();
  gc_mark_obj(state->stack);
  gc_mark_obj(state->env);

#if DEBUG & DEBUG_GC
  printf("GC:collect:frames: marking %lu frames.\n", state->fstack.length);
#endif
  for (u64 i = 0; i < state->fstack.length; ++i)
  {
    gc_mark_obj(state->fstack.frames[i].body);
    gc_mark_obj(state->fstack.frames[i].env);
  }

  size_t freed = gc_sweep();

#if DEBUG & DEBUG_GC
  BORDER();
#endif

  return freed;
}

void gc_stats(FILE *fp)
{
#if DEBUG & DEBUG_GC
  fprintf(fp,
          "stats\n"
          "\t%lu slots (%luB) over %lu %s allocated, of which %lu (%luB) are "
          "live.\n"
          "\tCollected %lu times.\n",
          (state->gc.pool.length * GC_CHUNK_DATA_SIZE) / 16,
          state->gc.pool.length * GC_CHUNK_DATA_SIZE, state->gc.pool.length,
          state->gc.pool.length == 1 ? "chunk" : "chunks",
          state->gc.metadata.slots_live, state->gc.metadata.slots_live * 16,
          state->gc.metadata.num_collections);
#else
  (void)fp;
#endif
}

/* Copyright (c) 2024 Anthony Bonkoski
 * Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
