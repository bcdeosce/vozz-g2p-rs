//! Wrapper sobre o BCDE-tagger (lib, in-process).
//!
//! O crate `bcde-tagger` expõe `Tagger::load(dir)` + `Tagger::tag(texto)`.
//! Aqui só encapsulamos para o resto do crate não depender dos tipos diretos.

pub use bcde_tagger::{Token, Tagger};

/// Carrega o tagger do diretório de dados.
pub fn carregar_tagger(dir: &str) -> Result<Tagger, String> {
    Tagger::load(dir).map_err(|e| format!("BCDE-tagger: {:?}", e))
}

/// Roda o tagger numa sentença.
pub fn anotar(tagger: &Tagger, texto: &str) -> Vec<Token> {
    tagger.tag(texto)
}
