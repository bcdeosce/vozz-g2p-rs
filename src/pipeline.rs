//! Pipeline de alto nível: texto bruto → chunks Piper.
//!
//! Junta normalização, splitter, G2P (com tagger + homógrafos),
//! conversão pra alfabeto Piper e chunking. Sem subprocesso.

use crate::g2p::{fonemizar, OpcoesFonemizar};
use crate::lexicon_homografos::LexiconHomografos;
use crate::normalize::{normalizar, OpcoesNormalizar};
use crate::piper::ipa_para_piper;
use crate::piper_pipeline::{preparar_chunks, Chunk};
use crate::splitter::dividir_em_sentencas;
use crate::tagger::Tagger;
use std::collections::HashMap;

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
    let normalizado = normalizar(texto, OpcoesNormalizar::default());
    let sentencas = dividir_em_sentencas(&normalizado);
    if sentencas.is_empty() {
        return Vec::new();
    }
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

    preparar_chunks(&ipa_piper)
}

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
