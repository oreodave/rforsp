/* optimiser.h: Optimiser for runtime
 * Created: 2026-07-05
 * Author: Aryadev Chavali
 * License: See end of file
 */

#ifndef OPTIMISER_H
#define OPTIMISER_H

#include "obj.h"

obj_t *optimise_closure_body(obj_t *body, obj_t *env);

#endif

/* Copyright (C) 2026 Aryadev Chavali

 * This program is distributed in the hope that it will be useful, but WITHOUT
 * ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 * FOR A PARTICULAR PURPOSE.  See the MIT License for details.

 * You may distribute and modify this code under the terms of the MIT License,
 * which you should have received a copy of along with this program.  If not,
 * please go to <https://opensource.org/license/MIT>.

 */
