//! Léxico de exceções pt-BR — modo isolado (1 palavra).
//!
//! Convenção IPA idêntica à do espeak-ng `pt-br`. Use `ɡ` (U+0261).
//! Strings em NFD.
//!
//! Os dados são carregados de `data/lexicon_palavra.json` em tempo de
//! build via `include_str!`. O arquivo JSON tem duas chaves:
//!
//! - `lexico`:    palavras de conteúdo (com acento primário).
//! - `cliticos`:  palavras átonas (sem acento primário).

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct ArquivoLexicon {
    lexico: HashMap<String, String>,
    cliticos: HashMap<String, String>,
}

static ARQUIVO: Lazy<ArquivoLexicon> = Lazy::new(|| {
    let json = include_str!("../data/lexicon_palavra.json");
    serde_json::from_str(json)
        .expect("JSON inválido em data/lexicon_palavra.json")
});

/// Consulta o léxico de palavras de conteúdo (modo isolado).
pub fn buscar_lexico(palavra: &str) -> Option<&'static str> {
    let chave = palavra.to_lowercase();
    ARQUIVO.lexico.get(&chave).map(|s| s.as_str())
}

/// Consulta clíticos átonos (modo isolado).
pub fn buscar_clitico(palavra: &str) -> Option<&'static str> {
    let chave = palavra.to_lowercase();
    ARQUIVO.cliticos.get(&chave).map(|s| s.as_str())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn lexicon_encontra_palavras_comuns() {
        assert!(buscar_lexico("não").is_some());
        assert!(buscar_lexico("também").is_some());
        assert!(buscar_lexico("exemplo").is_some());
        assert!(buscar_lexico("software").is_some());
        assert!(buscar_lexico("casa").is_none());
    }

    #[test]
    fn lexicon_palavra_ausente_retorna_none() {
        assert!(buscar_lexico("xyzabc").is_none());
        assert!(buscar_lexico("palavraqualquer").is_none());
        assert!(buscar_lexico("").is_none());
    }



    #[test]
    fn busca_e_case_insensitive() {
        assert_eq!(buscar_lexico("NÃO"), buscar_lexico("não"));
        assert_eq!(buscar_lexico("Não"), buscar_lexico("não"));
        assert_eq!(buscar_clitico("DE"), buscar_clitico("de"));
    }


    #[test]
    fn cliticos_restantes_sao_apenas_os_que_espeak_mantem_atonos() {
        // Após a migração, apenas `a`, `e`, `o`, `é` continuam em cliticos.
        // Todas as outras palavras funcionais foram movidas para `lexico`
        // com formas tônicas (isolado) e para `lexicon_contexto` com
        // formas átonas (contexto).
        assert!(buscar_clitico("a").is_some());
        assert!(buscar_clitico("e").is_some());
        assert!(buscar_clitico("o").is_some());
        assert!(buscar_clitico("é").is_some());
        assert!(buscar_clitico("de").is_none());
        assert!(buscar_clitico("que").is_none());
        assert!(buscar_clitico("para").is_none());
    }

    #[test]
    fn lexicon_encontra_palavras_migradas() {
        // As palavras funcionais migradas para `lexico` devem estar lá.
        assert!(buscar_lexico("de").is_some());
        assert!(buscar_lexico("que").is_some());
        assert!(buscar_lexico("para").is_some());
        assert!(buscar_lexico("ao").is_some());
        assert!(buscar_lexico("os").is_some());
        assert!(buscar_lexico("um").is_some());
    }

    #[test]
    #[allow(uncommon_codepoints)]
    fn de_isolado_e_tonico() {
        // `de` isolado → forma tônica `dʒˈy`.
        assert_eq!(buscar_lexico("de"), Some("dʒˈy"));
    }

    #[test]
    fn nao_ha_chaves_com_underscore() {
        assert!(buscar_clitico("a_").is_none());
        assert!(buscar_clitico("o_").is_none());
        assert!(buscar_clitico("a").is_some());
        assert!(buscar_clitico("o").is_some());
        assert!(buscar_lexico("às_").is_none());
        assert!(buscar_lexico("às").is_some());
    }

    #[test]
    fn o_artigo_isolado_e_upsilon() {
        // `o` continua sendo clítico átono (o espeak produz átono em citação).
        assert_eq!(buscar_clitico("o"), Some("ʊ"));
        // `os` foi movido para léxico com forma tônica.
        assert_eq!(buscar_lexico("os"), Some("ˈʊs"));
    }



}