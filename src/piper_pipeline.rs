//! Pipeline de alto nível: texto bruto → chunks Piper.
//!
//! Junta:
//!   1. normalize::normalizar    (datas, números, moedas)
//!   2. splitter::dividir_em_sentencas
//!   3. g2p::fonemizar           (usa tagger + homógrafos internamente)
//!   4. piper::ipa_para_piper
//!   5. piper_pipeline::preparar_chunks
//!
//! Nenhuma chamada a subprocesso. O tagger é uma referência direta
//! à lib `bcde_tagger::Tagger`.

use crate::g2p::{fonemizar, OpcoesFonemizar};
use crate::lexicon_homografos::LexiconHomografos;
use crate::normalize::{normalizar, OpcoesNormalizar};
use crate::piper::ipa_para_piper;
use crate::piper_pipeline::{preparar_chunks, Chunk};
use crate::splitter::dividir_em_sentencas;
use crate::tagger::Tagger;
use std::collections::HashMap;

/// Texto bruto → chunks prontos para síntese Piper.
///
/// `tagger`, `homografos`, `lexico` e `lexico_contexto` são opcionais.
/// Se o tagger for `None`, homógrafos não são desambiguados.
pub fn texto_para_chunks(
    texto: &str,
    tagger: Option<&Tagger>,
    homografos: Option<&LexiconHomografos>,
    lexico: Option<&HashMap<String, String>>,
    lexico_contexto: Option<&HashMap<String, String>>,
) -> Vec<Chunk> {
    if texto.trim().is_empty() {
        return Vec::new();
    }

    // 1. Normaliza
    let normalizado = normalizar(texto, OpcoesNormalizar::default());

    // 2. Divide em sentenças
    let sentencas = dividir_em_sentencas(&normalizado);
    if sentencas.is_empty() {
        return Vec::new();
    }

    // 3. Fonemiza cada sentença. O g2p consulta o tagger internamente.
    let opcoes = OpcoesFonemizar {
        normalizar: false,
        lexico,
        lexico_contexto,
        homografos,
        tagger,
    };

    let mut ipa_piper = String::new();
    for s in &sentencas {
        let ipa = fonemizar(s, &opcoes);
        let tokens = ipa_para_piper(&ipa);
        let pedaco: String = tokens.iter().map(|t| t.as_str()).collect();
        if !ipa_piper.is_empty() {
            ipa_piper.push(' ');
        }
        ipa_piper.push_str(pedaco.trim());
    }

    // 4. Chunking
    preparar_chunks(&ipa_piper)
}

/// Versão que devolve também a IPA completa (útil para debug).
pub fn texto_para_ipa(
    texto: &str,
    tagger: Option<&Tagger>,
    homografos: Option<&LexiconHomografos>,
    lexico: Option<&HashMap<String, String>>,
    lexico_contexto: Option<&HashMap<String, String>>,
) -> (String, Vec<Chunk>) {
    if texto.trim().is_empty() {
        return (String::new(), Vec::new());
    }

    let normalizado = normalizar(texto, OpcoesNormalizar::default());
    let sentencas = dividir_em_sentencas(&normalizado);

    let opcoes = OpcoesFonemizar {
        normalizar: false,
        lexico,
        lexico_contexto,
        homografos,
        tagger,
    };

    let mut ipa_piper = String::new();
    for s in &sentencas {
        let ipa = fonemizar(s, &opcoes);
        let tokens = ipa_para_piper(&ipa);
        let pedaco: String = tokens.iter().map(|t| t.as_str()).collect();
        if !ipa_piper.is_empty() { ipa_piper.push(' '); }
        ipa_piper.push_str(pedaco.trim());
    }

    let chunks = preparar_chunks(&ipa_piper);
    (ipa_piper, chunks)
}
