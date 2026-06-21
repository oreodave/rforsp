/* gc.c: Pool-based mark-sweep garbage collector for pairs and closures.
 * Created: 2026-06-21
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "gc.h"

static gc_chunk_t *chunk_list = NULL;
static void *free_list        = NULL;
static size_t alloc_cnt       = 0;
static size_t alloc_bytes     = 0;
static size_t threshold       = 64 * 16;

void gc_init(void)
{
}

void gc_stop(void)
{
  gc_chunk_t *c = chunk_list;
  while (c)
  {
    gc_chunk_t *next = c->next;
    free(c);
    c = next;
  }
  chunk_list  = NULL;
  free_list   = NULL;
  alloc_cnt   = 0;
  alloc_bytes = 0;
}

static gc_chunk_t *gc_new_chunk(void)
{
  /* aligned_alloc(4096, ...) also gives 16-byte alignment for slots */
  gc_chunk_t *c = aligned_alloc(4096, sizeof(gc_chunk_t));
  if (!c)
    FAIL("GC: failed to allocate chunk");
  memset(c->mark_bits, 0, sizeof(c->mark_bits));
  memset(c->alloc_bits, 0, sizeof(c->alloc_bits));

  /* Chain all new slots into the free list */
  for (size_t i = 0; i < GC_CHUNK_SLOTS; ++i)
  {
    void *slot     = c->data + i * 16;
    *(void **)slot = free_list;
    free_list      = slot;
  }

  c->next    = chunk_list;
  chunk_list = c;
  return c;
}

/* Locate which chunk owns a raw pointer. */
static gc_chunk_t *gc_find_chunk(void *raw)
{
  for (gc_chunk_t *c = chunk_list; c; c = c->next)
  {
    uint8_t *start = c->data;
    uint8_t *end   = start + GC_CHUNK_DATA_SIZE;
    if ((uint8_t *)raw >= start && (uint8_t *)raw < end)
      return c;
  }
  return NULL;
}

static size_t gc_slot_index(gc_chunk_t *c, void *raw)
{
  return ((uint8_t *)raw - c->data) / 16;
}

/* Bitmap helpers — work for both mark_bits and alloc_bits. */
static inline void gc_bit_set(uint64_t *bits, size_t idx)
{
  bits[idx / 64] |= (1ULL << (idx % 64));
}

static inline void gc_bit_clear(uint64_t *bits, size_t idx)
{
  bits[idx / 64] &= ~(1ULL << (idx % 64));
}

static inline bool gc_bit_test(const uint64_t *bits, size_t idx)
{
  return (bits[idx / 64] >> (idx % 64)) & 1;
}

__attribute__((noinline)) obj_t *gc_alloc(tag_t tag)
{
  if (alloc_bytes >= threshold)
    gc_collect();

  if (!free_list)
    gc_new_chunk();

  void *slot = free_list;
  free_list  = *(void **)slot;
  alloc_cnt++;
  alloc_bytes += 16;

  gc_chunk_t *c = gc_find_chunk(slot);
  size_t idx    = gc_slot_index(c, slot);
  gc_bit_set(c->alloc_bits, idx);

  return TAG_CANON(slot, tag);
}

void gc_mark_obj(obj_t *obj)
{
  if (!obj || IS_NIL(obj))
    return;

  if (!IS_ALLOC(obj))
    return;

  void *raw     = (void *)UNTAG(obj);
  gc_chunk_t *c = gc_find_chunk(raw);
  if (!c)
    return;

  size_t idx = gc_slot_index(c, raw);
  if (gc_bit_test(c->mark_bits, idx))
    return;

  gc_bit_set(c->mark_bits, idx);

  /* pair_t and clos_t both start with two obj_t* fields */
  obj_t **fields = (obj_t **)raw;
  gc_mark_obj(fields[0]);
  gc_mark_obj(fields[1]);
}

size_t gc_sweep(void)
{
  size_t freed = 0;
  for (gc_chunk_t *c = chunk_list; c; c = c->next)
  {
    for (size_t i = 0; i < GC_CHUNK_SLOTS; ++i)
    {
      if (!gc_bit_test(c->alloc_bits, i))
        continue;

      if (!gc_bit_test(c->mark_bits, i))
      {
        void *slot     = c->data + i * 16;
        *(void **)slot = free_list;
        free_list      = slot;
        gc_bit_clear(c->alloc_bits, i);
        freed++;
      }
    }
    memset(c->mark_bits, 0, sizeof(c->mark_bits));
  }

  alloc_cnt -= freed;
  alloc_bytes = alloc_cnt * 16;
  threshold   = alloc_bytes * 2;
  if (threshold < 64 * 16)
    threshold = 64 * 16;

  return freed;
}

size_t gc_collect(void)
{
  size_t freed = gc_sweep();
  return freed;
}

size_t gc_alloc_count(void)
{
  return alloc_cnt;
}
size_t gc_bytes_allocated(void)
{
  return alloc_bytes;
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
