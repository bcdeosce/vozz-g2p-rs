pub use bcde_tagger::{Tagger, Token, ALL_POS};

pub fn carregar_tagger(dir: &str) -> Result<Tagger, String> {
    Tagger::load(dir).map_err(|e| format!("BCDE-tagger: {:?}", e))
}

pub fn anotar(tagger: &Tagger, texto: &str) -> Vec<Token> {
    tagger.tag(texto)
}
