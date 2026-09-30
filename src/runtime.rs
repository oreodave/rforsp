//! rForsp language execution model.

mod variables;
pub use variables::{
    RuntimeVariable, RuntimeVariableId, RuntimeVariableRegistry,
};

mod primitives;
pub use primitives::{Primitive, PrimitiveId, PrimitiveRegistry};

// Top level assertion ensuring the default runtime variable names and primitive
// names together are a completely unique set.
const _: () = {
    let mut combined_names: [&str;
        variables::VARIABLE_NAMES.len() + primitives::PRIMITIVE_NAMES.len()] = ["";
        variables::VARIABLE_NAMES.len() + primitives::PRIMITIVE_NAMES.len()];

    let mut i = 0;
    while i < variables::VARIABLE_NAMES.len() {
        combined_names[i] = variables::VARIABLE_NAMES[i];
        i += 1;
    }

    let mut i = 0;
    while i < primitives::PRIMITIVE_NAMES.len() {
        combined_names[i + variables::VARIABLE_NAMES.len()] =
            primitives::PRIMITIVE_NAMES[i];
        i += 1;
    }

    // Uniqueness pass.
    let mut i = 0;
    while i < combined_names.len() {
        let x = combined_names[i].as_bytes();
        let mut j = i + 1;
        while j < combined_names.len() {
            let y = combined_names[j].as_bytes();
            // TODO(oreo)[2026-09-30 06:09]: Apparently even PartialEq checks
            // aren't const-stable.  Hence this bullshit.
            let mut equal = x.len() == y.len();
            let mut k = 0;
            while equal && k < x.len() {
                if x[k] != y[k] {
                    equal = false;
                }
                k += 1;
            }
            assert!(!equal, "expected unique symbols");

            j += 1;
        }
        i += 1;
    }
};
