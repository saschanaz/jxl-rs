#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// XXX: https://github.com/rust-lang/rust-bindgen/issues/1651
#![allow(deref_nullptr)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
