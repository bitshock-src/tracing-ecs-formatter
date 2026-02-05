#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]

mod format;
mod model;
mod visitor;

pub use format::EcsFormatter;
