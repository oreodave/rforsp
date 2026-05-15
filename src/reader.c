/* reader.c: Lexer and parser for Forsp source code.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "common.h"

char peek(void)
{
  if (state->input_pos == state->input_len)
    return 0;
  return state->input_str[state->input_pos];
}

void advance(void)
{
  assert(peek());
  state->input_pos++;
}

bool is_white(char c)
{
  return c == ' ' || c == '\t' || c == '\n';
}

bool is_directive(char c)
{
  return c == '\'' || c == '^' || c == '$';
}

bool is_punctuation(char c)
{
  return c == 0 || is_white(c) || is_directive(c) || c == '(' || c == ')' ||
         c == ';';
}

void skip_white_and_comments(void)
{
  char c = peek();
  if (c == 0)
    return;

  if (is_white(c))
  {
    advance();
    skip_white_and_comments();
  }

  if (c == ';')
  {
    advance();
    while (1)
    {
      char c = peek();
      if (c == 0)
        return;
      advance();
      if (c == '\n')
        break;
    }
    skip_white_and_comments();
  }
}

static bool parse_i64(const char *str, size_t len, int64_t *_out)
{
  static char buf[20];
  memcpy(buf, str, len);
  buf[len] = '\0';

  int64_t n = atoll(buf);
  if (n == 0 && 0 != strcmp(buf, "0"))
  {
    return false;
  }
  *_out = n;
  return true;
}

obj_t *read_scalar(void)
{
  size_t start = state->input_pos;
  while (!is_punctuation(peek()))
    advance();

  char *str = &state->input_str[start];
  size_t n  = state->input_pos - start;

  int64_t num;
  if (parse_i64(str, n, &num))
  {
    return make_num(num);
  }
  else
  {
    return intern(str, n);
  }
}

obj_t *read_list(void)
{
  if (!state->read_stack)
  {
    skip_white_and_comments();
    char c = peek();
    if (c == ')')
    {
      advance();
      return NULL;
    }
  }
  obj_t *first  = read();
  obj_t *second = read_list();
  return make_pair(first, second);
}

obj_t *read(void)
{
  obj_t *read_stack = state->read_stack;
  if (read_stack)
  {
    state->read_stack = cdr(read_stack);
    return car(read_stack);
  }

  skip_white_and_comments();

  char c = peek();
  if (!c)
    FAIL("End of input: could not read()");

  if (c == '\'')
  {
    advance();
    return state->atom_quote;
  }

  if (c == '^')
  {
    advance();
    obj_t *s          = NULL;
    s                 = make_pair(state->atom_push, s);
    s                 = make_pair(read_scalar(), s);
    s                 = make_pair(state->atom_quote, s);
    state->read_stack = s;
    return read();
  }

  if (c == '$')
  {
    advance();
    obj_t *s          = NULL;
    s                 = make_pair(state->atom_pop, s);
    s                 = make_pair(read_scalar(), s);
    s                 = make_pair(state->atom_quote, s);
    state->read_stack = s;
    return read();
  }

  if (c == '(')
  {
    advance();
    return read_list();
  }
  else
  {
    return read_scalar();
  }
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
