/* reader.c: Lexer and parser for Forsp source code.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "state.h"

char peek(void)
{
  if (state->input_pos == state->input_len)
    return 0;
  return state->input_str[state->input_pos];
}

char *stream_current(void)
{
  if (state->input_pos == state->input_len)
    return 0;
  return &state->input_str[state->input_pos];
}

void advance(void)
{
  assert(peek());
  state->input_pos++;
}

static const char *WHITESPACE  = "\n\t ";
static const char *PUNCTUATION = "\'^$()[];\n\t ";

void reader_error_position(void)
{
  // Derive col, line from state->input_pos
  size_t col, line, ind;
  for (col = 0, line = 1, ind = 0; ind <= state->input_pos; ++ind)
  {
    if (state->input_str[ind] == '\n')
    {
      ++line;
      col = 0;
    }
    else
    {
      ++col;
    }
  }
  fprintf(stderr, "%s:%lu:%lu: ", state->input_name, line, col);
}

#define READER_ERROR(...)    \
  do                         \
  {                          \
    reader_error_position(); \
    FAIL(__VA_ARGS__);       \
  } while (0)

void skip_white_and_comments(void)
{
  if (peek() == 0)
  {
    return;
  }

  state->input_pos += strspn(stream_current(), WHITESPACE);
  if (peek() == ';')
  {
    advance();
    state->input_pos += strcspn(stream_current(), "\n");
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
  size_t size = strcspn(stream_current(), PUNCTUATION);
  char *str   = stream_current();
  state->input_pos += size;

  int64_t num;
  if (parse_i64(str, size, &num))
  {
    return make_num(num);
  }
  else
  {
    // NOTE: quite liberal strings.
    return intern(str, size);
  }
}

obj_t *read_list(void)
{
  // NOTE: read_list only called when `(` encountered in read.  Thus, we record
  // the starting position for diagnostic purposes, then skip ahead.
  size_t start = state->input_pos;
  advance();

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

  c = peek();
  if (c != ')')
  {
    state->input_pos = start;
    READER_ERROR("Expected closing `)`");
  }
  else
  {
    advance();
    return root;
  }
}

obj_t *read_vec(void)
{
  // NOTE: read_vec only called when `[` encountered in read.  Thus, we record
  // the starting position for diagnostic purposes, then skip ahead.
  size_t start = state->input_pos;
  advance();

  obj_t *ovec = make_vec(0);
  vec_t *vec  = as_vec(ovec);

  char c = 0;
  skip_white_and_comments();
  for (c = peek(); (c && c != ']') || state->read_stack.length;
       skip_white_and_comments(), c = peek())
  {
    obj_t *item = read();
    vec_push(vec, item);
  }

  c = peek();
  if (c != ']')
  {
    state->input_pos = start;
    READER_ERROR("Expected closing `]`");
  }
  else
  {
    advance();
    return ovec;
  }
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
  switch (c)
  {
  case 0:
    READER_ERROR("End of input: could not read()");
    break;
  case '\'':
  {
    advance();
  }
    return state->atom_quote;
  case '^':
  {
    advance();
    obj_t *items[3] = {
        state->atom_push,
        read_scalar(),
        state->atom_quote,
    };
    vec_push_mult(&state->read_stack, items, 3);
  }
    return read();
  case '$':
  {
    advance();
    obj_t *items[3] = {
        state->atom_pop,
        read_scalar(),
        state->atom_quote,
    };
    vec_push_mult(&state->read_stack, items, 3);
  }
    return read();
  case '(':
    return read_list();
  case '[':
    return read_vec();
  default:
    return read_scalar();
  }
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
