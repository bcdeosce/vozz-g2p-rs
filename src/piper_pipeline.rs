//! Chunking inteligente. Não calcula pausas — só marca a pontuação
//! que fecha cada fragmento. A pausa fica a cargo do sintetizador.

use serde::Serialize;

pub const LIMITE_CHUNK: usize = 200;
pub const PUNCT_FRAC: &[char] = &['.', '!', '?', '…'];
pub const PUNCT_PAUSA: &[char] = &[',', ';', ':', '.', '!', '?', '…'];

#[derive(Serialize, Debug, Clone)]
pub struct Fragmento {
    pub ipa: String,
    /// Pontuação que fecha o fragmento. `None` = último, sem pausa.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub punct: Option<char>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Chunk {
    pub fragments: Vec<Fragmento>,
}

pub fn chunk_inteligente(ipa: &str, limite: usize) -> Vec<String> {
    let ipa = ipa.trim();
    if ipa.is_empty() { return Vec::new(); }
    if ipa.chars().count() <= limite { return vec![ipa.to_string()]; }

    let mut partes: Vec<String> = Vec::new();
    let mut buf = String::new();
    for c in ipa.chars() {
        buf.push(c);
        if PUNCT_FRAC.contains(&c) {
            let s = buf.trim().to_string();
            if !s.is_empty() { partes.push(s); }
            buf.clear();
        }
    }
    let resto = buf.trim();
    if !resto.is_empty() { partes.push(resto.to_string()); }

    let mut grupos: Vec<String> = Vec::new();
    let mut atual = String::new();
    for p in partes {
        let cand = if atual.is_empty() { p.clone() } else { format!("{} {}", atual, p) };
        if cand.chars().count() > limite && !atual.is_empty() {
            grupos.push(atual.trim().to_string());
            atual = p;
        } else {
            atual = cand;
        }
    }
    if !atual.trim().is_empty() { grupos.push(atual.trim().to_string()); }
    grupos
}

pub fn segmentar_por_pausa(ipa: &str) -> Vec<(String, Option<char>)> {
    let mut frags = Vec::new();
    let mut buf = String::new();
    for c in ipa.chars() {
        if PUNCT_PAUSA.contains(&c) {
            let s = buf.trim().to_string();
            if !s.is_empty() { frags.push((s, Some(c))); }
            buf.clear();
        } else {
            buf.push(c);
        }
    }
    let resto = buf.trim();
    if !resto.is_empty() { frags.push((resto.to_string(), None)); }
    frags
}

pub fn preparar_chunks(ipa_piper: &str) -> Vec<Chunk> {
    let chunks_ipa = chunk_inteligente(ipa_piper, LIMITE_CHUNK);
    chunks_ipa.into_iter().map(|c| {
        let frags = segmentar_por_pausa(&c);
        Chunk {
            fragments: frags.into_iter().map(|(ipa, punct)| Fragmento { ipa, punct }).collect(),
        }
    }).collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn chunk_curto_e_unico() {
        assert_eq!(chunk_inteligente("ʊ xˈatʊ.", 200).len(), 1);
    }

    #[test]
    fn marca_punct() {
        let chunks = preparar_chunks("ʊ xˈatʊ, xoˈew. mˈais");
        assert_eq!(chunks.len(), 1);
        let frags = &chunks[0].fragments;
        assert_eq!(frags[0].punct, Some(','));
        assert_eq!(frags[1].punct, Some('.'));
        assert_eq!(frags[2].punct, None);
    }
}
