/* primitives.h: Fixed primitives of the Forsp language.
 * Created: 2026-06-17
 * Author: Aryadev Chavali
 * License: See end of file
 */

#ifndef PRIMITIVES_H
#define PRIMITIVES_H

#include "state.h"

/******************************************************************************
 * Primitives                                                                 *
 ******************************************************************************/

// required
void prim_push(obj_t **env);
void prim_pop(obj_t **env);
void prim_eq(obj_t **_);
void prim_cons(obj_t **_);
void prim_car(obj_t **_);
void prim_cdr(obj_t **_);
void prim_cswap(obj_t **_);
void prim_tag(obj_t **_);
void prim_read(obj_t **_);
void prim_print(obj_t **_);

// helpful
void prim_stack(obj_t **_);
void prim_env(obj_t **env);
void prim_sub(obj_t **_);
void prim_mul(obj_t **_);
void prim_nand(obj_t **_);
void prim_lsh(obj_t **_);
void prim_rsh(obj_t **_);

// vectors/container helpers
void prim_copy(obj_t **_);
void prim_length(obj_t **_);
void prim_vmake(obj_t **_);
void prim_vpush(obj_t **_);
void prim_vpop(obj_t **_);
void prim_vswap(obj_t **_);
void prim_vget(obj_t **_);
void prim_vset(obj_t **_);

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
