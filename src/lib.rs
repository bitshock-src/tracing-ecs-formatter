#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::needless_doctest_main)]
#![doc = include_str!("../README.md")]

mod format;
mod model;
mod visitor;

pub use format::EcsFormatter;
