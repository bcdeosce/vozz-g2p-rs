//! Léxico de exceções pt-BR — modo contexto (2+ palavras).
//!
//! Convenção IPA idêntica à do espeak-ng `pt-br`. Use `ɡ` (U+0261).
//! Strings em NFD.
//!
//! Os dados são carregados de `data/lexicon_contexto.json` em tempo de
//! build via `include_str!`. O arquivo JSON tem duas chaves:
//!
//! - `cliticos_contexto`:  clíticos com pronúncia alterada em contexto.
//! - `lexico_contexto`:    palavras de conteúdo com pronúncia alterada
//!                         em contexto (rebaixamento de acento, etc.).

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct ArquivoLexicon {
    cliticos_contexto: HashMap<String, String>,
    #[serde(default)]
    lexico_contexto: HashMap<String, String>,
}

static ARQUIVO: Lazy<ArquivoLexicon> = Lazy::new(|| {
    let json = include_str!("../data/lexicon_contexto.json");
    serde_json::from_str(json)
        .expect("JSON inválido em data/lexicon_contexto.json")
});

/// Consulta clíticos átonos em contexto de sentença.
pub fn buscar_clitico_contexto(palavra: &str) -> Option<&'static str> {
    let chave = palavra.to_lowercase();
    ARQUIVO.cliticos_contexto.get(&chave).map(|s| s.as_str())
}

/// Consulta léxico de palavras de conteúdo em contexto de sentença.
pub fn buscar_lexico_contexto(palavra: &str) -> Option<&'static str> {
    let chave = palavra.to_lowercase();
    ARQUIVO.lexico_contexto.get(&chave).map(|s| s.as_str())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cliticos_contexto_encontram_palavras() {
        assert!(buscar_clitico_contexto("que").is_some());
        assert!(buscar_clitico_contexto("na").is_some());
        assert!(buscar_clitico_contexto("para").is_some());
        assert!(buscar_clitico_contexto("onde").is_some());
        assert!(buscar_clitico_contexto("ser").is_some());
        assert!(buscar_clitico_contexto("você").is_some());
        assert!(buscar_clitico_contexto("os").is_some());
        assert!(buscar_clitico_contexto("as").is_some());
        assert!(buscar_clitico_contexto("por").is_some());
        assert!(buscar_clitico_contexto("com").is_some());
    }

    #[test]
    fn cliticos_contexto_que_correto() {
        // `que` continua hardcoded — o espeak sempre produz `ky` para ele.
        assert_eq!(buscar_clitico_contexto("que"), Some("ky"));
    }

    #[test]
    fn cliticos_contexto_uma_secundario() {
        assert_eq!(buscar_clitico_contexto("uma"), Some("ˌumæ"));
    }

    #[test]
    fn cliticos_contexto_case_insensitive() {
        assert_eq!(buscar_clitico_contexto("Que"), buscar_clitico_contexto("que"));
        assert_eq!(buscar_clitico_contexto("PARA"), buscar_clitico_contexto("para"));
        assert_eq!(buscar_clitico_contexto("Os"), buscar_clitico_contexto("os"));
    }

    #[test]
    fn lexico_contexto_encontra_rebaixamentos() {
        assert!(buscar_lexico_contexto("não").is_some());
        assert!(buscar_lexico_contexto("foram").is_some());
        assert!(buscar_lexico_contexto("sempre").is_some());
        assert!(buscar_lexico_contexto("minha").is_some());
    }

    #[test]
    fn lexico_contexto_ausente_retorna_none() {
        assert!(buscar_lexico_contexto("casa").is_none());
        assert!(buscar_lexico_contexto("xyzabc").is_none());
    }
}