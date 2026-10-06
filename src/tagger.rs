//! Wrapper sobre o BCDE-tagger.
//!
//! Isola o resto do código dos detalhes da API do crate externo.
//! Se o nome dos métodos/campos mudar no `bcde-tagger`, só este
//! arquivo precisa ser ajustado.
//!
//! API usada:
//!
//! ```ignore
//! let t = Tagger::load("data")?;
//! let tokens = t.tag("texto");
//! for token in tokens {
//!     // token.word:    String
//!     // token.sense:   Option<String>
//!     // token.upos:    String
//!     // token.diacritic: String
//! }
//! ```

use std::path::Path;

pub use bcde_tagger::Tagger;

/// Token anotado pelo tagger.
#[derive(Debug, Clone)]
pub struct TokenTagger {
    pub word: String,
    pub upos: String,
    pub diacritic: String,
    pub sense: Option<String>,
}

/// Carrega o tagger de um diretório de dados.
pub fn carregar_tagger<P: AsRef<Path>>(dir: P) -> Result<Tagger, String> {
    Tagger::load(dir.as_ref()).map_err(|e| format!("BCDE-tagger: {:?}", e))
}

/// Anota uma sentença inteira.
///
/// Se a API do crate tiver outro nome de método, ajuste aqui.
pub fn anotar(tagger: &Tagger, texto: &str) -> Vec<TokenTagger> {
    tagger
        .tag(texto)
        .into_iter()
        .map(|t| TokenTagger {
            word: t.word,
            upos: t.upos,
            diacritic: t.diacritic,
            sense: t.sense,
        })
        .collect()
}
