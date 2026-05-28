/* main.c: Entrypoint, setup, and file loading.
 * Created: 2026-05-15
 * Author: Aryadev Chavali
 * License: See end of file
 */

#include "common.h"

static char *load_file(const char *filename)
{
  FILE *fp = fopen(filename, "r");
  if (!fp)
    FAIL("Failed to open file: '%s'\n", filename);

  fseek(fp, 0, SEEK_END);
  size_t file_size = ftell(fp);
  fseek(fp, 0, SEEK_SET);

  char *mem = malloc(file_size + 1);
  size_t n  = fread(mem, 1, file_size, fp);
  if (n != file_size)
    FAIL("Failed to read file");

  mem[file_size] = 0;
  return mem;
}

void setup(const char *input_path)
{
  memset(state, 0, sizeof(state));
  state->input_str = load_file(input_path);
  state->input_len = strlen(state->input_str);
  state->input_pos = 0;

  state->atom_true  = intern("t", 1);
  state->atom_quote = intern("quote", 5);
  state->atom_push  = intern("push", 4);
  state->atom_pop   = intern("pop", 3);

  obj_t *env = NULL;
  env        = env_define_prim(env, "push", &prim_push);
  env        = env_define_prim(env, "pop", &prim_pop);
  env        = env_define_prim(env, "cons", &prim_cons);
  env        = env_define_prim(env, "car", &prim_car);
  env        = env_define_prim(env, "cdr", &prim_cdr);
  env        = env_define_prim(env, "eq", &prim_eq);
  env        = env_define_prim(env, "cswap", &prim_cswap);
  env        = env_define_prim(env, "tag", &prim_tag);
  env        = env_define_prim(env, "read", &prim_read);
  env        = env_define_prim(env, "print", &prim_print);

  env = env_define_prim(env, "stack", &prim_stack);
  env = env_define_prim(env, "env", &prim_env);
  env = env_define_prim(env, "-", &prim_sub);
  env = env_define_prim(env, "*", &prim_mul);
  env = env_define_prim(env, "nand", &prim_nand);
  env = env_define_prim(env, "<<", &prim_lsh);
  env = env_define_prim(env, ">>", &prim_rsh);

  state->env = env;
}

int main(int argc, char *argv[])
{
  if (argc != 2)
  {
    fprintf(stderr, "usage: %s <path>\n", argv[0]);
    return 1;
  }
  setup(argv[1]);

  obj_t *obj = read();
  compute(obj, state->env);

  return 0;
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
