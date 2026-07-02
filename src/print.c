/* print.c: Pretty-printing of Forsp objects.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "state.h"

void print_recurse(obj_t *obj);

void print_list_tail(obj_t *obj)
{
  if (obj == NULL)
  {
    printf(")");
    return;
  }
  if (IS_PAIR(obj))
  {
    printf(" ");
    print_recurse(DIRECT_CAR(obj));
    print_list_tail(DIRECT_CDR(obj));
  }
  else
  {
    printf(" . ");
    print_recurse(obj);
    printf(")");
  }
}

void print_recurse(obj_t *obj)
{
  switch (get_tag(obj))
  {
  case TAG_NIL:
    printf("()");
    break;
  case TAG_ATOM:
    printf("%s", as_atom(obj));
    break;
  case TAG_NUM:
    printf("%" PRId64, as_num(obj));
    break;
  case TAG_PAIR:
  {
    printf("(");
    print_recurse(DIRECT_CAR(obj));
    print_list_tail(DIRECT_CDR(obj));
  }
  break;
  case TAG_CLOS:
  {
    printf("CLOSURE<");
    clos_t *clos = DIRECT_UNTAG(obj, clos_t *);
    print_recurse(clos->body);
    printf(", %p>", (void *)clos->env);
  }
  break;
  case TAG_VEC:
  {
    printf("[");
    vec_t *vec = as_vec(obj);
    for (size_t i = 0; i < vec->length; ++i)
    {
      print(vec->items[i]);
      if (i != vec->length - 1)
        printf(" ");
    }
    printf("]");
  }
  break;
  case TAG_PRIM:
  {
    // NOTE: Illegal trick.  I should be deported for this.
    union
    {
      void (*funcptr)(obj_t **);
      void *ptr;
    } u = {as_prim(obj)};
    printf("PRIM<%p>", u.ptr);
  }
  break;
  }
}

void print(obj_t *obj)
{
  print_recurse(obj);
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
