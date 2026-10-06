//! Reexport da API do BCDE-tagger.
//!
//! O BCDE-tagger expõe `[lib]` com nome `bcde_tagger`.
//! Aqui só reexportamos para o resto do crate usar `crate::tagger::Tagger`.

pub use bcde_tagger::{Tagger, Token, ALL_POS};
