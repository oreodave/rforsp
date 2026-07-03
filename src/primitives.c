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

void prim_nand(obj_t **_)
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

void prim_length(obj_t **_)
{
  (void)_;
  obj_t *item = pop();
  if (!IS_CONTAINER(item))
  {
    FAIL("prim_length: Expected container, got %p\n", (void *)item);
  }

  u64 size = 0;
  if (IS_VEC(item))
  {
    size = DIRECT_UNTAG(item, vec_t *)->length;
  }
  else if (IS_PAIR(item))
  {
    for (obj_t *top = item; top; top = DIRECT_CDR(top), ++size)
    {
      continue;
    }
  }

  push(make_num(size));
}

void prim_vmake(obj_t **_)
{
  (void)_;
  obj_t *size    = pop();
  obj_t *new_vec = make_vec(as_num(size));
  push(new_vec);
}

void prim_vpush(obj_t **_)
{
  (void)_;
  obj_t *item = pop();
  obj_t *vec  = pop();
  vec_push(as_vec(vec), item);
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
