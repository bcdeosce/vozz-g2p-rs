//! Chunking inteligente + cálculo de pausas para síntese Piper.
//!
//! Recebe o IPA completo (já processado pelo vozz) e devolve uma lista
//! de chunks prontos para o cliente sintetizar:
//!
//! - Se o IPA total ≤ LIMITE_CHUNK chars → 1 chunk só (entonação natural).
//! - Se > LIMITE_CHUNK → agrupa pedaços cortados em `.`/`!`/`?`/`…` até
//!   chegar em ~LIMITE_CHUNK chars por chunk.
//!
//! Dentro de cada chunk, o IPA é segmentado nas pontuações de pausa
//! (`,` `;` `:` `.` `!` `?` `…`), e cada fragmento carrega a duração de
//! silêncio a inserir depois dele.

use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;

/// Tamanho máximo (em chars de IPA) por chunk de síntese.
pub const LIMITE_CHUNK: usize = 200;

/// Pontuações que podem terminar um chunk.
pub const PUNCT_FRAC: &[char] = &['.', '!', '?', '…'];

/// Pontuações que geram pausa interna dentro de um chunk.
pub const PUNCT_PAUSA: &[char] = &[',', ';', ':', '.', '!', '?', '…'];

/// Duração base das pausas em ms, por pontuação (modo neutro).
pub static PAUSAS_BASE: Lazy<HashMap<char, u32>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert(',', 180);
    m.insert(';', 280);
    m.insert(':', 350);
    m.insert('.', 500);
    m.insert('!', 450);
    m.insert('?', 450);
    m.insert('…', 750);
    m
});

/// Presets de emoção: `(length_scale, fator_pausa)`.
///
/// - `length_scale` < 1 acelera, > 1 desacelera.
/// - `fator_pausa` multiplica as durações base das pausas.
pub static EMOCOES: Lazy<HashMap<&'static str, (f32, f32)>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("neutro",    (1.00, 1.00));
    m.insert("ansioso",   (0.86, 0.40));
    m.insert("cansado",   (1.25, 1.80));
    m.insert("triste",    (1.08, 1.55));
    m.insert("empolgado", (0.92, 0.55));
    m.insert("calmo",     (1.05, 1.20));
    m
});

// ---------------------------------------------------------------------------
// Tipos públicos
// ---------------------------------------------------------------------------

#[derive(Serialize, Debug, Clone)]
pub struct Fragmento {
    /// IPA do fragmento, sem a pontuação que o fecha.
    pub ipa: String,
    /// Pausa em ms a inserir DEPOIS deste fragmento (0 = nenhuma).
    pub pausa_ms: u32,
}

#[derive(Serialize, Debug, Clone)]
pub struct Chunk {
    pub fragments: Vec<Fragmento>,
    /// `length_scale` a usar na síntese deste chunk.
    pub length_scale: f32,
    /// Pausa em ms a inserir DEPOIS deste chunk (antes do próximo).
    pub pausa_apos_ms: u32,
}

// ---------------------------------------------------------------------------
// API
// ---------------------------------------------------------------------------

/// Decide em quantos chunks o IPA deve ser sintetizado.
///
/// - Se `ipa.chars().count() <= limite` → devolve `[ipa]` (síntese única).
/// - Senão → corta em `PUNCT_FRAC` e agrupa pedaços até `limite` chars.
pub fn chunk_inteligente(ipa: &str, limite: usize) -> Vec<String> {
    let ipa = ipa.trim();
    if ipa.is_empty() {
        return Vec::new();
    }
    if ipa.chars().count() <= limite {
        return vec![ipa.to_string()];
    }

    // 1. Separa em pedaços terminando em pontuação final.
    let mut partes: Vec<String> = Vec::new();
    let mut buf = String::new();
    for c in ipa.chars() {
        buf.push(c);
        if PUNCT_FRAC.contains(&c) {
            let s = buf.trim().to_string();
            if !s.is_empty() {
                partes.push(s);
            }
            buf.clear();
        }
    }
    let resto = buf.trim();
    if !resto.is_empty() {
        partes.push(resto.to_string());
    }

    // 2. Agrupa pedaços até o limite.
    let mut grupos: Vec<String> = Vec::new();
    let mut atual = String::new();
    for p in partes {
        let cand = if atual.is_empty() {
            p.clone()
        } else {
            format!("{} {}", atual, p)
        };
        if cand.chars().count() > limite && !atual.is_empty() {
            grupos.push(atual.trim().to_string());
            atual = p;
        } else {
            atual = cand;
        }
    }
    if !atual.trim().is_empty() {
        grupos.push(atual.trim().to_string());
    }
    grupos
}

/// Corta o IPA nos caracteres de pausa, devolvendo `(fragmento, punct)`.
/// O último fragmento (sem pontuação) vem com `None`.
pub fn segmentar_por_pausa(ipa: &str) -> Vec<(String, Option<char>)> {
    let mut frags: Vec<(String, Option<char>)> = Vec::new();
    let mut buf = String::new();
    for c in ipa.chars() {
        if PUNCT_PAUSA.contains(&c) {
            let s = buf.trim().to_string();
            if !s.is_empty() {
                frags.push((s, Some(c)));
            }
            buf.clear();
        } else {
            buf.push(c);
        }
    }
    let resto = buf.trim();
    if !resto.is_empty() {
        frags.push((resto.to_string(), None));
    }
    frags
}

/// Pausa em ms após uma pontuação, modulada pela emoção.
pub fn calcular_pausa(punct: char, emocao: &str) -> u32 {
    let (_, fp) = EMOCOES.get(emocao).copied().unwrap_or((1.0, 1.0));
    let base = PAUSAS_BASE.get(&punct).copied().unwrap_or(0);
    (base as f32 * fp).round() as u32
}

/// `length_scale` para a emoção dada.
pub fn length_scale(emocao: &str) -> f32 {
    EMOCOES.get(emocao).map(|(ls, _)| *ls).unwrap_or(1.0)
}

/// Pipeline completo: IPA → chunks prontos para síntese.
pub fn preparar_chunks(ipa_completo: &str, emocao: &str) -> Vec<Chunk> {
    let ls = length_scale(emocao);
    let pausa_entre = calcular_pausa('.', emocao);

    let chunks_ipa = chunk_inteligente(ipa_completo, LIMITE_CHUNK);
    let n = chunks_ipa.len();
    let mut saida = Vec::with_capacity(n);

    for (i, c) in chunks_ipa.into_iter().enumerate() {
        let frags = segmentar_por_pausa(&c);
        let mut fragmentos = Vec::with_capacity(frags.len());
        for (frag, punct) in frags {
            let pausa_ms = match punct {
                Some(p) => calcular_pausa(p, emocao),
                None => 0,
            };
            fragmentos.push(Fragmento { ipa: frag, pausa_ms });
        }
        let pausa_apos_ms = if i + 1 < n { pausa_entre } else { 0 };
        saida.push(Chunk {
            fragments: fragmentos,
            length_scale: ls,
            pausa_apos_ms,
        });
    }
    saida
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn chunk_curto_e_unico() {
        let chunks = chunk_inteligente("ʊ xˈatʊ xoˈew.", 200);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn chunk_longo_quebra() {
        let ipa = "a. ".repeat(100); // ~300 chars
        let chunks = chunk_inteligente(&ipa, 200);
        assert!(chunks.len() >= 2);
        for c in &chunks {
            assert!(c.chars().count() <= 200, "chunk tem {} chars", c.chars().count());
        }
    }

    #[test]
    fn segmentar_por_pausa_basico() {
        let frags = segmentar_por_pausa("ʊ xˈatʊ, xoˈew. mˈais");
        assert_eq!(frags.len(), 3);
        assert_eq!(frags[0].1, Some(','));
        assert_eq!(frags[1].1, Some('.'));
        assert_eq!(frags[2].1, None);
    }

    #[test]
    fn pausa_emocao_ansioso_reduz() {
        let neutro = calcular_pausa(',', "neutro");
        let ansioso = calcular_pausa(',', "ansioso");
        assert!(ansioso < neutro);
    }

    #[test]
    fn preparar_chunks_emocao_triste() {
        let ipa = "ʊ xˈatʊ xoˈew. ˈmais aˈinda.";
        let chunks = preparar_chunks(ipa, "triste");
        assert_eq!(chunks.len(), 1);
        assert!((chunks[0].length_scale - 1.08).abs() < 0.01);
        // pausa após "." em triste = 500 * 1.55 = 775
        assert_eq!(chunks[0].fragments[0].pausa_ms, 775);
    }
}
