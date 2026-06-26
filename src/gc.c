/* gc.c: Pool-based mark-sweep garbage collector for pairs and closures.
 * Created: 2026-06-21
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "gc.h"
#include "state.h"

/******************************************************************************
 * Bitmap Helpers                                                             *
 ******************************************************************************/

static inline void bitmap_set(u64 *bits, size_t idx)
{
  bits[idx / 64] |= (1ULL << (idx % 64));
}

static inline void bitmap_clear(u64 *bits, size_t idx)
{
  bits[idx / 64] &= ~(1ULL << (idx % 64));
}

static inline bool bitmap_test(const u64 *bits, size_t idx)
{
  return (bits[idx / 64] >> (idx % 64)) & 1;
}

/******************************************************************************
 * Free list Helpers                                                          *
 ******************************************************************************/

static inline void gc_free_list_push(void *slot)
{
  ((pair_t *)slot)->car = state->gc.free_list;
  state->gc.free_list   = slot;
}

static inline void *gc_free_list_pop()
{
  pair_t *free_slot = state->gc.free_list;
  if (free_slot)
    state->gc.free_list = free_slot->car;
  return free_slot;
}

/******************************************************************************
 * GC Methods                                                                 *
 ******************************************************************************/

void gc_init()
{
  memset(&state->gc, 0, sizeof(state->gc));
  state->gc.metadata.threshold = GC_THRESHOLD_DEFAULT;
}

void gc_stop()
{
  for (size_t i = 0; i < state->gc.pool.length; ++i)
  {
    free(state->gc.pool.chunks[i]);
  }
  free(state->gc.pool.chunks);
  memset(&state->gc, 0, sizeof(state->gc));
}

void gc_reset()
{
  gc_stop();
  gc_init();
}

/** Construct a new chunk and push it onto the pool.
 */
static gc_chunk_t *gc_new_chunk(void)
{
  // aligned_alloc(4096, ...) gives 16-byte alignment for slots
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
    gc_free_list_push(slot);
  }

  // Push onto the chunk array in the pool.
  if (!state->gc.pool.capacity)
  {
    state->gc.pool.capacity = 1;
    state->gc.pool.chunks   = malloc(sizeof(*state->gc.pool.chunks));
  }
  else if (state->gc.pool.capacity - state->gc.pool.length == 0)
  {
    state->gc.pool.capacity *= 2;
    state->gc.pool.chunks =
        realloc(state->gc.pool.chunks,
                sizeof(*state->gc.pool.chunks) * state->gc.pool.capacity);
  }

  if (!state->gc.pool.chunks)
  {
    FAIL("GC: failed to reallocate pool of chunks");
  }

  state->gc.pool.chunks[state->gc.pool.length++] = c;

  return c;
}

static inline bool gc_ptr_in_chunk(gc_chunk_t *chunk, void *raw_ptr)
{
  auto start = chunk->data;
  auto end   = start + GC_CHUNK_DATA_SIZE;
  return (u8 *)raw_ptr >= start && (u8 *)raw_ptr < end;
}

static inline size_t gc_ptr_slot_in_chunk(gc_chunk_t *chunk, void *raw_ptr)
{
  return ((u8 *)raw_ptr - chunk->data) / 16;
}

/** Locate which chunk owns a raw pointer, returning it.
 * If no chunk is found, return NULL.

 * The slot position of the pointer in the chunk is stored in `slot_id`.
 */
static gc_chunk_t *gc_find_chunk(void *raw_ptr, size_t *slot_id)
{
  for (size_t i = 0; i < state->gc.pool.length; ++i)
  {
    auto c = state->gc.pool.chunks[i];
    if (gc_ptr_in_chunk(c, raw_ptr))
    {
      *slot_id = gc_ptr_slot_in_chunk(c, raw_ptr);
      return c;
    }
  }
  return NULL;
}

__attribute__((noinline)) obj_t *gc_alloc(tag_t tag)
{
  if (state->gc.metadata.alloc_live * 16 >= state->gc.metadata.threshold)
  {
    gc_collect();
  }
  if (!state->gc.free_list)
  {
    gc_new_chunk();
  }

  void *slot = gc_free_list_pop();
  state->gc.metadata.alloc_live++;

  size_t idx    = 0;
  gc_chunk_t *c = gc_find_chunk(slot, &idx);
  bitmap_set(c->live_bits, idx);

  return TAG_CANON(slot, tag);
}

void gc_mark_obj(obj_t *obj)
{
  if (!IS_ALLOC(obj))
    return;

  void *raw     = (void *)UNTAG(obj);
  size_t idx    = 0;
  gc_chunk_t *c = gc_find_chunk(raw, &idx);

  if (!c || bitmap_test(c->mark_bits, idx))
    return;

  bitmap_set(c->mark_bits, idx);

  // pair_t and clos_t both start with two obj_t* fields
  obj_t **fields = (obj_t **)raw;
  gc_mark_obj(fields[0]);
  gc_mark_obj(fields[1]);
}

size_t gc_sweep(void)
{
  size_t freed = 0;
  for (size_t i = 0; i < state->gc.pool.length; ++i)
  {
    gc_chunk_t *c = state->gc.pool.chunks[i];
    for (size_t w = 0; w < GC_CHUNK_MARK_WORDS; ++w)
    {
      // We only want to free those that are both live and unmarked.
      u64 to_free  = c->live_bits[w] & ~c->mark_bits[w];
      u64 word_pos = w * 64;
      while (to_free)
      {
        // Find the lowest bit which is nonzero through a single hardware inst.
        int bit = __builtin_ctzll(to_free);
        // Clear the lowest bit.
        to_free &= to_free - 1;

        // Access the slot and put it on the free list.
        void *slot = c->data + ((word_pos + bit) * 16);
        gc_free_list_push(slot);
        bitmap_clear(c->live_bits, word_pos + bit);
        freed++;
      }
    }
    memset(c->mark_bits, 0, sizeof(c->mark_bits));
  }

  state->gc.metadata.alloc_live -= freed;
  state->gc.metadata.threshold =
      MAX(GC_THRESHOLD_DEFAULT, state->gc.metadata.alloc_live * 32);

#if DEBUG & DEBUG_GC
  printf("GC:sweep: freed %lu bytes\n", freed * 16);
#endif
  return freed;
}

constexpr size_t GC_STACK_MARCH_LIMIT = 512;
static inline void gc_mark_stack_march(void)
{
  void *sp;
  __asm__ volatile("mov %%rsp, %0" : "=r"(sp));
  void **end = (void **)((u8 *)sp + GC_STACK_MARCH_LIMIT);

#if (DEBUG & DEBUG_GC) != 0
  printf("GC:collect:stack_march: iterating from start=%p -> end=%p\n", sp,
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
        printf("\tMarking allocation %p => %p\n", (void *)p, raw);
#endif
        gc_mark_obj(maybe);
      }
    }
  }
}

size_t gc_collect(void)
{
#if DEBUG & DEBUG_GC
  ++state->gc.metadata.num_collections;
#endif

  gc_mark_stack_march();
  gc_mark_obj(state->stack);
  gc_mark_obj(state->env);

#if DEBUG & DEBUG_GC
  printf("GC:collect:frames: marking %lu frames.\n", state->frame_depth);
#endif
  for (i64 i = 0; i < state->frame_depth; ++i)
  {
    gc_mark_obj(state->frames[i].body);
    gc_mark_obj(state->frames[i].env);
  }

  size_t freed = gc_sweep();

#if DEBUG & DEBUG_GC
  BORDER();
#endif

  return freed;
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
