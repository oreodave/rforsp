/* gc_test.c: Standalone tests for the pool-based mark-sweep GC.
 * Created: 2026-06-21
 * Author: Aryadev Chavali
 * License: See end of file
 *
 * Tests the GC in isolation — does NOT depend on the interpreter.
 */

#include "../src/common.h"
#include "../src/gc.h"
#include "../src/obj.h"
#include "../src/state.h"
#include "libtest.h"

// We're not linking with main thus we need to generate our own state here.
state_t state[1];

static obj_t *mkpair(obj_t *car, obj_t *cdr)
{
  obj_t *p  = gc_alloc(TAG_PAIR);
  pair_t *r = as_pair(p);
  r->car    = car;
  r->cdr    = cdr;
  return p;
}

static obj_t *mkclos(obj_t *body, obj_t *env)
{
  obj_t *p  = gc_alloc(TAG_CLOS);
  clos_t *r = as_clos(p);
  r->body   = body;
  r->env    = env;
  return p;
}

static void test_alloc_and_collect(void)
{
  TSTART("alloc and collect dead objects");
  gc_reset();

  size_t before = gc_alloc_count();

  // Allocate five pairs, no roots -> all garbage
  for (int i = 0; i < 5; ++i)
    mkpair(NULL, NULL);

  TCHECK(gc_alloc_count() == before + 5, "alloc count should be +5");

  size_t freed = gc_collect();
  TCHECK(freed == 5, "all 5 should be collected");
  TCHECK(gc_alloc_count() == 0, "alloc count should be 0");
  TCHECK(gc_bytes_allocated() == 0, "bytes should be 0");
  TPASS();

done:
  gc_reset();
}

static void test_live_object_survives(void)
{
  TSTART("marked object survives collection");
  gc_reset();

  obj_t *live = mkpair(NULL, NULL);
  (void)mkpair(NULL, NULL); // garbage
  size_t before = gc_alloc_count();
  TCHECK(before == 2, "should have 2 allocations");

  // Mark only 'live'
  gc_mark_obj(live);
  size_t freed = gc_collect();
  TCHECK(freed == 1, "1 dead object should be collected");

  // live should still be a valid pair, dead should be reclaimed (we check this
  // indirectly: after collection, the pointer for dead may be in the free list
  // - but we can still allocate and verify)
  pair_t *p = as_pair(live);
  TCHECK(p->car == NULL && p->cdr == NULL, "live pair fields intact");

  TCHECK(gc_alloc_count() == 1, "1 object remaining");
  TPASS();

done:
  gc_reset();
}

static void test_chain_survives(void)
{
  TSTART("chained objects rooted at head survive");
  gc_reset();

  // Build a 3-element chain: head -> mid -> tail -> NULL
  obj_t *tail = mkpair(NULL, NULL);
  obj_t *mid  = mkpair(NULL, tail);
  obj_t *head = mkpair(mid, NULL);

  // Also allocate garbage
  mkpair(NULL, NULL);

  size_t before = gc_alloc_count();
  TCHECK(before == 4, "4 allocations");

  // Root only the head
  gc_mark_obj(head);
  size_t freed = gc_collect();
  TCHECK(freed == 1, "1 garbage object collected");

  // All 3 chain pairs should survive
  TCHECK(gc_alloc_count() == 3, "3 objects survive");

  pair_t *hp = as_pair(head);
  TCHECK(hp->car == mid, "head.car is mid");
  pair_t *mp = as_pair(mid);
  TCHECK(mp->cdr == tail, "mid.cdr is tail");
  TPASS();

done:
  gc_reset();
}

static void test_self_referencing_garbage(void)
{
  TSTART("self-referencing cycle is collected");
  gc_reset();

  // Create a pair that points to itself -> cycle with no external roots
  obj_t *self = mkpair(NULL, NULL);
  {
    pair_t *p = as_pair(self);
    p->car    = self;
    p->cdr    = self;
  }

  size_t before = gc_alloc_count();
  TCHECK(before == 1, "1 allocation");

  size_t freed = gc_collect();
  TCHECK(freed == 1, "self-referencing cycle should be collected");
  TCHECK(gc_alloc_count() == 0, "no objects remain");
  TPASS();

done:
  gc_reset();
}

static void test_mutual_cycle_garbage(void)
{
  TSTART("mutual cycle (pair A<->B) is collected");
  gc_reset();

  obj_t *a = mkpair(NULL, NULL);
  obj_t *b = mkpair(NULL, a);
  {
    pair_t *p_a = as_pair(a);
    p_a->cdr    = b;
  }

  TCHECK(gc_alloc_count() == 2, "2 allocations");

  size_t freed = gc_collect();
  TCHECK(freed == 2, "mutual cycle should be collected");
  TCHECK(gc_alloc_count() == 0, "no objects remain");
  TPASS();

done:
  gc_reset();
}

static void test_closure_survives(void)
{
  TSTART("marked closure with captured env survives");
  gc_reset();

  obj_t *env  = mkpair(NULL, NULL); // dummy env
  obj_t *body = mkpair(NULL, NULL); // dummy body
  obj_t *clos = mkclos(body, env);

  TCHECK(gc_alloc_count() == 3, "3 allocations");

  // Mark only the closure — body and env reached transitively
  gc_mark_obj(clos);
  size_t freed = gc_collect();
  TCHECK(freed == 0, "body and env reachable via closure mark");
  TCHECK(gc_alloc_count() == 3, "all 3 survive");

  clos_t *c = as_clos(clos);
  TCHECK(c->body == body, "closure.body intact");
  TCHECK(c->env == env, "closure.env intact");
  TPASS();

done:
  gc_reset();
}

static void test_free_list_reuse(void)
{
  TSTART("freed slots are reused by subsequent allocs");
  gc_reset();

  // Allocate 5, free them all
  (void)mkpair(NULL, NULL);
  (void)mkpair(NULL, NULL);
  (void)mkpair(NULL, NULL);
  (void)mkpair(NULL, NULL);
  (void)mkpair(NULL, NULL);
  TCHECK(gc_alloc_count() == 5, "5 allocs");

  size_t freed = gc_collect();
  TCHECK(freed == 5, "all freed");

  // Allocate again — should come from free list, no new chunk needed
  obj_t *x = mkpair(NULL, NULL);
  (void)mkpair(NULL, NULL); // will be collected below
  TCHECK(gc_alloc_count() == 2, "2 allocs from free list");
  TCHECK(gc_bytes_allocated() == 32, "32 bytes");

  // Only mark x
  gc_mark_obj(x);
  gc_collect();
  TCHECK(gc_alloc_count() == 1, "1 survives");
  pair_t *xp = as_pair(x);
  TCHECK(xp->car == NULL, "x.car intact after reuse cycle");
  TPASS();

done:
  gc_reset();
}

static void test_multiple_collections(void)
{
  TSTART("multiple collect cycles with mixed live/dead");
  gc_reset();

  for (int round = 0; round < 3; ++round)
  {
    obj_t *keep = mkpair(NULL, NULL);
    // allocate some garbage.
    mkpair(NULL, NULL);
    mkpair(NULL, NULL);

    // After the first round, the previous keep is still allocated (allocated
    // but unmarked), so count is 1 + 3 = 4 in later rounds.
    size_t expect = (round == 0) ? 3 : 4;
    TCHECK(gc_alloc_count() == expect, "allocs before GC");

    gc_mark_obj(keep);
    size_t freed        = gc_collect();
    size_t expect_freed = (round == 0) ? 2 : 3;
    TCHECK(freed == expect_freed, "dead objects collected");
    TCHECK(gc_alloc_count() == 1, "1 survives");
    pair_t *kp = as_pair(keep);
    TCHECK(kp->car == NULL, "keep intact");
  }
  TPASS();

done:
  gc_reset();
}

static void test_closure_cycle_garbage(void)
{
  TSTART("closure capturing itself is collected");
  gc_reset();

  // clos -> { body: (pair pointing to clos), env: (pair pointing to clos) }
  // This is the classic closure-captures-its-own-environment cycle.
  obj_t *clos = mkclos(NULL, NULL);
  obj_t *env  = mkpair(clos, NULL);
  obj_t *body = mkpair(clos, NULL);

  {
    clos_t *cclos = as_clos(clos);
    cclos->body   = body;
    cclos->env    = env;
  }

  TCHECK(gc_alloc_count() == 3, "3 allocs for cycle");

  // No roots -> all should be collected
  size_t freed = gc_collect();
  TCHECK(freed == 3, "closure cycle collected");
  TCHECK(gc_alloc_count() == 0, "nothing remains");
  TPASS();

done:
  gc_reset();
}

int main(void)
{
  printf("GC Tests\n");

  test_alloc_and_collect();
  test_live_object_survives();
  test_chain_survives();
  test_self_referencing_garbage();
  test_mutual_cycle_garbage();
  test_closure_survives();
  test_free_list_reuse();
  test_multiple_collections();
  test_closure_cycle_garbage();

  TEST_RESULT();
  return (tests_pass == tests_run) ? 0 : 1;
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
