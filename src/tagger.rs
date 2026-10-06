//! Wrapper sobre a API do BCDE-tagger (dependência git).
//!
//! O crate `bcde-tagger` expõe `[lib] name = "bcde_tagger"`.
//! Aqui reexportamos `Tagger`, `Token`, `ALL_POS` e oferecemos um
//! `carregar_tagger` que encapsula `Tagger::load` para o worker.

pub use bcde_tagger::{Tagger, Token, ALL_POS};

/// Carrega o tagger de um diretório de dados.
///
/// O BCDE-tagger espera um diretório com `tabelas_v3.json`,
/// `crf_weights.json`, `diacriticos_table.json`, `resolver_v7.json`.
pub fn carregar_tagger(dir: &str) -> Result<Tagger, String> {
    Tagger::load(dir).map_err(|e| format!("{:?}", e))
}
