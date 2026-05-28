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
  if (peek() == 0)
    return;

  while (is_white(peek()))
  {
    advance();
  }
  if (peek() == ';')
  {
    advance();
    for (char c = peek(); c != '\n'; advance(), c = peek())
    {
      if (c == 0)
      {
        return;
      }
    }
    // We need to do a recur here to kill any further whitespace or comments.
    skip_white_and_comments();
  }
}

static bool parse_i64(const char *str, size_t len, int64_t *_out)
{
  static char buf[20];
  memcpy(buf, str, len);
  buf[len] = '\0';

  int64_t n = atoll(buf);
  if (n == 0 && strcmp(buf, "0"))
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
    // NOTE: quite liberal strings.
    return intern(str, n);
  }
}

obj_t *read_list(void)
{
  obj_t *root = NULL, *cur = NULL;
  char c = 0;
  skip_white_and_comments();
  for (c = peek(); (c && c != ')') || state->read_stack.length;
       skip_white_and_comments(), c = peek())
  {
    obj_t *item = read();
    if (!root)
    {
      root = make_pair(item, NULL);
      cur  = root;
    }
    else
    {
      pair_t *pair = as_pair(cur);
      pair->cdr    = make_pair(item, NULL);
      cur          = pair->cdr;
    }
  }
  if (c != ')')
  {
    FAIL("Expected closing brace");
  }
  else
  {
    advance();
  }
  return root;
}

obj_t *read(void)
{
  if (state->read_stack.length)
  {
    obj_t *ret = NULL;
    // NOTE: We've verified length is non zero, but it's best to assert.
    assert(vec_try_pop(&state->read_stack, &ret));
    return ret;
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
  else if (c == '^')
  {
    // TODO: Remove the read stack idea entirely.
    advance();
    obj_t *items[3] = {
        state->atom_push,
        read_scalar(),
        state->atom_quote,
    };
    vec_push_mult(&state->read_stack, items, 3);
    return read();
  }
  else if (c == '$')
  {
    advance();
    obj_t *items[3] = {
        state->atom_pop,
        read_scalar(),
        state->atom_quote,
    };
    vec_push_mult(&state->read_stack, items, 3);
    return read();
  }
  else if (c == '(')
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
