/* primitives.c: Core and extra primitive functions.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "primitives.h"

void prim_push(obj_t **env)
{
  auto key = pop();
  push(env_find(*env, key));
}

void prim_pop(obj_t **env)
{
  auto k = pop();
  auto v = pop();
  *env   = env_define(*env, k, v);
}

void prim_eq(obj_t **_)
{
  (void)_;
  push(obj_equal(pop(), pop()) ? state->atom_true : NULL);
}

void prim_cons(obj_t **_)
{
  (void)_;
  auto a = pop();
  auto b = pop();
  push(make_pair(a, b));
}

void prim_car(obj_t **_)
{
  (void)_;
  push(car(pop()));
}

void prim_cdr(obj_t **_)
{
  (void)_;
  push(cdr(pop()));
}

void prim_cswap(obj_t **_)
{
  (void)_;
  if (pop() == state->atom_true)
  {
    auto a = pop();
    auto b = pop();
    push(a);
    push(b);
  }
}

void prim_tag(obj_t **_)
{
  (void)_;
  push(make_num(GET_TAG(pop())));
}

void prim_read(obj_t **_)
{
  (void)_;
  push(read());
}

void prim_print(obj_t **_)
{
  (void)_;
  print(pop());
  printf("\n");
}

void prim_stack(obj_t **_)
{
  (void)_;
  push(state->stack);
}

void prim_env(obj_t **env)
{
  push(*env);
}

void prim_nil(obj_t **_)
{
  (void)_;
  push(NULL);
}

void prim_add(obj_t **_)
{
  (void)_;
  auto b = pop();
  auto a = pop();
  push(make_num(as_num(a) + as_num(b)));
}

void prim_sub(obj_t **_)
{
  (void)_;
  auto b = pop();
  auto a = pop();
  push(make_num(as_num(a) - as_num(b)));
}

void prim_mul(obj_t **_)
{
  (void)_;
  auto b = pop();
  auto a = pop();
  push(make_num(as_num(a) * as_num(b)));
}

void prim_div(obj_t **_)
{
  (void)_;
  auto b = pop();
  auto a = pop();
  push(make_num(as_num(a) / as_num(b)));
}

void prim_bitwise_and(obj_t **_)
{
  (void)_;
  auto b = pop();
  auto a = pop();
  push(make_num(as_num(a) & as_num(b)));
}

void prim_bitwise_or(obj_t **_)
{
  (void)_;
  auto b = pop();
  auto a = pop();
  push(make_num(as_num(a) | as_num(b)));
}

void prim_bitwise_nand(obj_t **_)
{
  (void)_;
  auto b = pop();
  auto a = pop();
  push(make_num(~(as_num(a) & as_num(b))));
}

void prim_lsh(obj_t **_)
{
  (void)_;
  auto b = pop();
  auto a = pop();
  push(make_num(as_num(a) << as_num(b)));
}

void prim_rsh(obj_t **_)
{
  (void)_;
  auto b = pop();
  auto a = pop();
  push(make_num(as_num(a) >> as_num(b)));
}

void prim_rec(obj_t **_)
{
  (void)_;
  auto closure = pop();
  if (!IS_CLOS(closure))
  {
    FAIL("Expected closure on stack for `rec`");
  }
  push(TAG_TYPE(UNTAG(closure), REC));
}

void prim_copy(obj_t **_)
{
  (void)_;
  auto item = pop();
  if (!IS_CONTAINER(item))
  {
    push(item);
  }
  else if (IS_PAIR(item))
  {
    push(make_pair(DIRECT_CAR(item), DIRECT_CDR(item)));
  }
  else if (IS_VEC(item))
  {
    auto v        = as_vec(item);
    auto copy     = make_vec(v->length);
    auto new_v    = as_vec(copy);
    new_v->length = v->length;
    memcpy(new_v->items, v->items, sizeof(*v->items) * v->length);
    push(copy);
  }
}

void prim_length(obj_t **_)
{
  (void)_;
  auto item = pop();
  if (!IS_CONTAINER(item))
  {
    FAIL("prim_length: Expected container, got %p", (void *)item);
  }

  u64 size = 0;
  if (IS_VEC(item))
  {
    size = TYPED_UNTAG(item, vec_t *)->length;
  }
  else if (IS_PAIR(item))
  {
    for (auto top = item; top; top = DIRECT_CDR(top), ++size)
    {
      continue;
    }
  }

  push(make_num(size));
}

void prim_vmake(obj_t **_)
{
  (void)_;
  auto osize = pop();
  if (!IS_NUM(osize) || as_num(osize) < 0)
    FAIL("prim_vmake: Expected unsigned number for size, got %p",
         (void *)osize);

  auto size    = as_num(osize);
  auto new_vec = make_vec(size);
  // Set the length of the vector to the requested size.
  TYPED_UNTAG(new_vec, vec_t *)->length = size;
  push(new_vec);
}

void prim_vpush(obj_t **_)
{
  (void)_;
  auto item = pop();
  auto vec  = pop();
  if (!IS_VEC(vec))
    FAIL("prim_vpush: Expected vector, got %p", (void *)vec);
  vec_push(as_vec(vec), item);
  push(vec);
}

void prim_vpop(obj_t **_)
{
  (void)_;
  auto vec   = pop();
  obj_t *ret = NULL;
  if (!IS_VEC(vec))
    FAIL("prim_vpop: Expected vector, got %p", (void *)vec);
  else if (!vec_try_pop(as_vec(vec), &ret))
    FAIL("prim_vpop: Vector (%p) has 0 length.", (void *)vec);

  push(ret);
  push(vec);
}

void prim_vswap(obj_t **_)
{
  (void)_;
  auto vec = pop();
  if (!IS_VEC(vec))
    FAIL("prim_vswap: Expected vector, got %p", (void *)vec);
  auto old_stack = state->stack;
  state->stack   = vec;
  push(old_stack);
}

void prim_vget(obj_t **_)
{
  (void)_;
  auto index = pop();
  auto vec   = pop();
  if (!IS_VEC(vec))
    FAIL("prim_vget: Expected vector, got %p", (void *)vec);
  else if (!IS_NUM(index))
    FAIL("prim_vget: Expected number, got %p", (void *)index);

  auto ind = as_num(index);
  auto v   = as_vec(vec);
  if (ind >= v->length)
    FAIL("prim_vget: Index (%ld) out of bounds for vec (%u)", ind, v->length);

  push(v->items[ind]);
}

void prim_vset(obj_t **_)
{
  (void)_;
  auto item  = pop();
  auto index = pop();
  auto vec   = pop();
  if (!IS_VEC(vec))
    FAIL("prim_vget: Expected vector, got %p", (void *)vec);
  else if (!IS_NUM(index))
    FAIL("prim_vget: Expected number, got %p", (void *)index);

  auto ind = as_num(index);
  auto v   = as_vec(vec);
  if (ind >= v->length)
    FAIL("prim_vget: Index (%ld) out of bounds for vec (%u)", ind, v->length);

  v->items[ind] = item;
  push(vec);
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
