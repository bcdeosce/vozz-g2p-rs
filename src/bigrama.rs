//! Regras de bigrama contextual para desambiguação de homógrafos.
//!
//! Um bigrama é uma sequência de 3 palavras: a palavra-alvo seguida
//! (ou precedida) por duas palavras que discriminam o sentido. O
//! classificador Naive Bayes olha apenas 1 palavra para cada lado;
//! este módulo cobre os casos em que a palavra decisiva está a 2
//! posições de distância.
//!
//! Exemplo:
//!
//!   "O molho de palha serviu para estofar a cama."
//!     palavra = "molho", next1 = "de", next2 = "palha" → bundle
//!
//!   "Serviu o prato com um molho de cogumelos selvagens."
//!     palavra = "molho", next1 = "de", next2 = "cogumelos" → sauce
//!
//! Os dados vêm de `data/bigramas.json`, gerado por `gerar_bigramas.py`,
//! e são embutidos no binário via `include_str!` — não há I/O de runtime
//! nem dependência de CWD.
//!
//! Formato do JSON:
//!
//! ```json
//! {
//!   "next": { "palavra|w1|w2": "sense|pos", ... },
//!   "prev": { "palavra|w2|w1": "sense|pos", ... }
//! }
//! ```
//!
//! O valor pode vir com `|pos` (formato do gerador Python). Este módulo
//! devolve só o `sense`, que é o que o `lexicon_homografos.json` usa
//! como chave.

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Deserialize)]
struct BigramasJson {
    #[serde(default)]
    next: HashMap<String, String>,
    #[serde(default)]
    prev: HashMap<String, String>,
}

static BIGRAMAS: Lazy<BigramasJson> = Lazy::new(|| {
    let raw = include_str!("../data/bigramas.json");
    serde_json::from_str(raw).expect("data/bigramas.json inválido")
});

/// Conjunto de palavras-alvo com pelo menos um bigrama cadastrado.
///
/// Pré-computado uma vez, na primeira chamada. Serve para `tem_bigrama`
/// rodar em O(1) — essa função é chamada para **toda** palavra de uma
/// sentença via `Homografos::tem_regra`, então não pode varrer o mapa.
static PALAVRAS_COM_BIGRAMA: Lazy<HashSet<String>> = Lazy::new(|| {
    let mut set: HashSet<String> = HashSet::with_capacity(
        BIGRAMAS.next.len().saturating_add(BIGRAMAS.prev.len()),
    );
    for chave in BIGRAMAS.next.keys().chain(BIGRAMAS.prev.keys()) {
        if let Some(palavra) = chave.split('|').next() {
            if !palavra.is_empty() {
                set.insert(palavra.to_string());
            }
        }
    }
    set
});

/// Extrai o sentido puro de um valor `"sense|pos"` armazenado no JSON.
///
/// O gerador Python grava `sense|pos`, mas o `lexicon_homografos.json`
/// é chaveado apenas por `sense`. Esta função descarta o `|pos`.
#[inline]
fn sentido_puro(valor: &str) -> &str {
    match valor.find('|') {
        Some(pos) => &valor[..pos],
        None => valor,
    }
}

/// Verifica se a palavra tem algum bigrama cadastrado (forward ou backward).
pub fn tem_bigrama(palavra: &str) -> bool {
    PALAVRAS_COM_BIGRAMA.contains(palavra)
}

/// Verifica bigrama forward. Devolve o sentido se a palavra-alvo é
/// seguida por exatamente `next1` e `next2`.
pub fn verificar_next(
    palavra: &str,
    next1: &str,
    next2: &str,
) -> Option<&'static str> {
    let chave = format!("{}|{}|{}", palavra, next1, next2);
    BIGRAMAS
        .next
        .get(chave.as_str())
        .map(|v| sentido_puro(v.as_str()))
}

/// Verifica bigrama backward. Devolve o sentido se a palavra-alvo é
/// precedida por exatamente `prev2` e `prev1`.
pub fn verificar_prev(
    palavra: &str,
    prev2: &str,
    prev1: &str,
) -> Option<&'static str> {
    let chave = format!("{}|{}|{}", palavra, prev2, prev1);
    BIGRAMAS
        .prev
        .get(chave.as_str())
        .map(|v| sentido_puro(v.as_str()))
}

/// Número de entradas carregadas, por direção.
/// Útil para log de startup / diagnóstico.
pub fn estatisticas() -> (usize, usize) {
    (BIGRAMAS.next.len(), BIGRAMAS.prev.len())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tem_bigrama_detecta_palavras_presentes_no_json() {
        // Se algum destes falhar, o `data/bigramas.json` está vazio ou
        // foi gerado sem essas palavras — investigue o gerador.
        assert!(tem_bigrama("molho"), "molho ausente do bigramas.json");
        assert!(tem_bigrama("colher"), "colher ausente do bigramas.json");
        assert!(tem_bigrama("bola"), "bola ausente do bigramas.json");
    }

    #[test]
    fn tem_bigrama_nao_dispara_para_palavra_comum() {
        assert!(!tem_bigrama("casa"));
        assert!(!tem_bigrama("palavra_inexistente_xyz"));
    }

    #[test]
    fn verificar_next_encontra_par_conhecido() {
        // Depende do conteúdo de data/bigramas.json. Se o gerador mudar
        // os thresholds, estes podem deixar de existir — ajuste conforme.
        if let Some(s) = verificar_next("molho", "de", "palha") {
            assert_eq!(s, "bundle");
        }
        if let Some(s) = verificar_next("molho", "de", "cogumelos") {
            assert_eq!(s, "sauce");
        }
    }

    #[test]
    fn verificar_next_retorna_none_para_desconhecido() {
        assert!(verificar_next("palavra_inexistente_xyz", "de", "palha").is_none());
        assert!(verificar_next("molho", "zzz", "yyy").is_none());
    }

    #[test]
    fn verificar_prev_encontra_par_conhecido() {
        if let Some(s) = verificar_prev("colher", "começou", "a") {
            assert_eq!(s, "harvest");
        }
    }

    #[test]
    fn verificar_prev_retorna_none_para_desconhecido() {
        assert!(verificar_prev("palavra_inexistente_xyz", "começou", "a").is_none());
    }

    #[test]
    fn sentido_puro_remove_pos() {
        assert_eq!(sentido_puro("harvest|VERB"), "harvest");
        assert_eq!(sentido_puro("bundle"), "bundle");
        assert_eq!(sentido_puro(""), "");
    }
}
