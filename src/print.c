/* print.c: Pretty-printing of Forsp objects.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "common.h"

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
    print_recurse(car(obj));
    print_list_tail(cdr(obj));
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
  if (obj == NULL)
  {
    printf("()");
    return;
  }
  switch (GET_TAG(obj))
  {
  case TAG_ATOM:
    printf("%s", as_atom(obj));
    break;
  case TAG_NUM:
    printf("%" PRId64, as_num(obj));
    break;
  case TAG_PAIR:
  {
    printf("(");
    print_recurse(car(obj));
    print_list_tail(cdr(obj));
  }
  break;
  case TAG_CLOS:
  {
    printf("CLOSURE<");
    clos_t *clos = as_clos(obj);
    print_recurse(clos->body);
    printf(", %p>", (void *)clos->env);
  }
  break;
  case TAG_PRIM:
  {
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
  printf("\n");
}

/* Copyright (c) 2024 Anthony Bonkoski
 * Copyright (C) 2026 Aryadev Chavali
 *
 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.
 *
 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.
 */
