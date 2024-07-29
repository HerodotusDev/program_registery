use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};

use cairo_vm::types::{builtin_name::BuiltinName, layout_name::LayoutName};

type NTraceColumns = u32;

// See https://github.com/starkware-libs/cairo-lang/blob/0e4dab8a6065d80d1c726394f5d9d23cb451706a/src/starkware/cairo/lang/instances.py
lazy_static! {
    pub static ref LAYOUT_INFO: HashMap<LayoutName, (NTraceColumns, HashSet<BuiltinName>)> = {
        let mut m = HashMap::new();
        m.insert(
            LayoutName::starknet_with_keccak,
            (
                15,
                HashSet::from([
                    BuiltinName::pedersen,
                    BuiltinName::range_check,
                    BuiltinName::ecdsa,
                    BuiltinName::bitwise,
                    BuiltinName::ec_op,
                    BuiltinName::keccak,
                    BuiltinName::poseidon,
                ]),
            ),
        );
        m.insert(
            LayoutName::recursive,
            (
                10,
                HashSet::from([
                    BuiltinName::pedersen,
                    BuiltinName::range_check,
                    BuiltinName::bitwise,
                ]),
            ),
        );
        m.insert(
            LayoutName::starknet,
            (
                10,
                HashSet::from([
                    BuiltinName::pedersen,
                    BuiltinName::range_check,
                    BuiltinName::ecdsa,
                    BuiltinName::bitwise,
                    BuiltinName::ec_op,
                    BuiltinName::poseidon,
                ]),
            ),
        );
        m.insert(
            LayoutName::recursive_with_poseidon,
            (
                8,
                HashSet::from([
                    BuiltinName::pedersen,
                    BuiltinName::range_check,
                    BuiltinName::bitwise,
                    BuiltinName::poseidon,
                ]),
            ),
        );
        m
    };
}
