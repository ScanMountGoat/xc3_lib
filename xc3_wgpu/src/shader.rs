#![allow(dead_code)]

// Include the bindings generated by build.rs.
// Not modifying the src directory makes this crate easier to publish.
include!(concat!(env!("OUT_DIR"), "/shader.rs"));
