/* libtest.h: Test helper macros for the Forsp test suite.
 * Created: 2026-06-21
 * Author: Aryadev Chavali
 * License: See end of file
 */

#ifndef LIBTEST_H
#define LIBTEST_H

#include <stdio.h>

#define ANSI_GREEN "\033[32m"
#define ANSI_RED   "\033[31m"
#define ANSI_RESET "\033[0m"

static int tests_run  = 0;
static int tests_pass = 0;

#define TSTART(name)              \
  do                              \
  {                               \
    tests_run++;                  \
    printf("  %-50s ... ", name); \
    fflush(stdout);               \
  } while (0)

#define TPASS()                                \
  do                                           \
  {                                            \
    tests_pass++;                              \
    printf(ANSI_GREEN "PASS" ANSI_RESET "\n"); \
  } while (0)

#define TFAIL(msg)                                    \
  do                                                  \
  {                                                   \
    printf(ANSI_RED "FAIL: %s" ANSI_RESET "\n", msg); \
  } while (0)

#define TCHECK(cond, msg) \
  do                      \
  {                       \
    if (!(cond))          \
    {                     \
      TFAIL(msg);         \
      goto done;          \
    }                     \
  } while (0)

#define TEST_RESULT()                                          \
  do                                                           \
  {                                                            \
    printf("\n%d / %d tests passed\n", tests_pass, tests_run); \
  } while (0)

#endif

/* Copyright (c) 2024 Anthony Bonkoski
 * Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
