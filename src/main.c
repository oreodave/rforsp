/* main.c: Entrypoint, setup, and file loading.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "common.h"
#include "compute.h"
#include "state.h"

static char *load_file(const char *filename, size_t *const size)
{
  FILE *fp = fopen(filename, "r");
  if (!fp)
    FAIL("Failed to open file: '%s'\n", filename);

  fseek(fp, 0, SEEK_END);
  *size = ftell(fp);
  fseek(fp, 0, SEEK_SET);

  char *mem = malloc(*size + 1);
  size_t n  = fread(mem, 1, *size, fp);
  fclose(fp);

  if (n != *size)
    FAIL("Failed to read file");

  mem[*size] = 0;
  return mem;
}

// Allocate the state variable in this code unit.
state_t state[1];

int main(int argc, char *argv[])
{
  if (argc != 2)
  {
    fprintf(stderr, "usage: %s <path>\n", argv[0]);
    return 1;
  }

  state_init();

  state->input_name = argv[1];
  state->input_str  = load_file(argv[1], &state->input_len);
  state->input_pos  = 0;

  obj_t *obj = read();
  compute(obj, state->env);

#if DEBUG & DEBUG_GC
  BORDER();
  printf("GC:exit stats\n"
         "\t%luB over %lu %s allocated, of which %luB are live.\n"
         "\tCollected %lu times.\n",
         state->gc.pool.length * GC_CHUNK_DATA_SIZE, state->gc.pool.length,
         state->gc.pool.length == 1 ? "chunk" : "chunks",
         state->gc.metadata.alloc_live * 16,
         state->gc.metadata.num_collections);
#endif

  // free(state->input_str);
  // state_stop();

  return 0;
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
