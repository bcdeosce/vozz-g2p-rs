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
//! O NB sozinho não distingue esses dois: em ambos,
//! `prev_word = "o"` ou `prev_word = "um"`, e `next_word = "de"`.
//! A palavra que resolve está em `+2`.

/// Bigramas forward: `(palavra, próxima_1, próxima_2) → sentido`.
///
/// Ambos os lados seguintes são normalizados para minúsculas.
pub static BIGRAMAS_NEXT: &[(&str, &str, &str, &str)] = &[
    // ─── molho ───
    ("molho", "de", "palha",       "bundle"),
    ("molho", "de", "flores",      "bundle"),
    ("molho", "de", "varinhas",    "bundle"),
    ("molho", "de", "menta",       "bundle"),
    ("molho", "de", "especiarias", "bundle"),
    ("molho", "de", "cogumelos",   "sauce"),
    ("molho", "de", "wasabi",      "sauce"),

    // ─── bola ───
    ("bola", "de", "carne",   "loaf"),
    ("bola", "de", "futebol", "ball"),
    ("bola", "de", "pelo",    "ball"),

    // ─── polo ───
    ("polo", "de", "energia",    "hub"),
    ("polo", "de", "diversão",   "sport_polo"),
    ("polo", "de", "esporte",    "sport_polo"),
    ("polo", "de", "desenvolvimento", "hub"),
];

/// Bigramas backward: `(palavra, anterior_2, anterior_1) → sentido`.
///
/// Útil quando o verbo causativo (que revela que a palavra-alvo é
/// verbo) está a 2 posições antes. Exemplo:
///
///   "A equipa começou a colher depoimentos..."
///     palavra = "colher", anterior_2 = "começou", anterior_1 = "a"
///     → harvest
pub static BIGRAMAS_PREV: &[(&str, &str, &str, &str)] = &[
    // ─── colher (verbo) ───
    ("colher", "começou",   "a", "harvest"),
    ("colher", "começaram", "a", "harvest"),
    ("colher", "comecei",   "a", "harvest"),
    ("colher", "começamos", "a", "harvest"),
    ("colher", "vamos",     "a", "harvest"),
    ("colher", "vou",       "a", "harvest"),

    // ─── atropelo (verbo) ───
    ("atropelo", "eu", "que", "running_over"),
];

/// Verifica se a palavra tem algum bigrama cadastrado.
pub fn tem_bigrama(palavra: &str) -> bool {
    for (palavra_alvo, _, _, _) in BIGRAMAS_NEXT {
        if *palavra_alvo == palavra {
            return true;
        }
    }
    for (palavra_alvo, _, _, _) in BIGRAMAS_PREV {
        if *palavra_alvo == palavra {
            return true;
        }
    }
    false
}

/// Verifica bigrama forward. Devolve o sentido se a palavra-alvo é
/// seguida por exatamente `next1` e `next2`.
pub fn verificar_next(
    palavra: &str,
    next1: &str,
    next2: &str,
) -> Option<&'static str> {
    for (palavra_alvo, proxima_1, proxima_2, sentido) in BIGRAMAS_NEXT {
        if *palavra_alvo == palavra && *proxima_1 == next1 && *proxima_2 == next2 {
            return Some(sentido);
        }
    }
    None
}

/// Verifica bigrama backward. Devolve o sentido se a palavra-alvo é
/// precedida por exatamente `prev2` e `prev1`.
pub fn verificar_prev(
    palavra: &str,
    prev2: &str,
    prev1: &str,
) -> Option<&'static str> {
    for (palavra_alvo, anterior_2, anterior_1, sentido) in BIGRAMAS_PREV {
        if *palavra_alvo == palavra && *anterior_2 == prev2 && *anterior_1 == prev1 {
            return Some(sentido);
        }
    }
    None
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn bigrama_next_encontra_palavra_conhecida() {
        assert_eq!(verificar_next("molho", "de", "palha"), Some("bundle"));
        assert_eq!(verificar_next("molho", "de", "cogumelos"), Some("sauce"));
        assert_eq!(verificar_next("bola", "de", "carne"), Some("loaf"));
        assert_eq!(verificar_next("bola", "de", "futebol"), Some("ball"));
    }

    #[test]
    fn bigrama_next_retorna_none_para_combinacao_desconhecida() {
        assert_eq!(verificar_next("molho", "de", "inexistente"), None);
        assert_eq!(verificar_next("palavra", "de", "palha"), None);
    }

    #[test]
    fn bigrama_prev_encontra_palavra_conhecida() {
        assert_eq!(verificar_prev("colher", "começou", "a"), Some("harvest"));
        assert_eq!(verificar_prev("colher", "vamos", "a"), Some("harvest"));
    }

    #[test]
    fn bigrama_prev_retorna_none_para_combinacao_desconhecida() {
        assert_eq!(verificar_prev("colher", "quis", "a"), None);
        assert_eq!(verificar_prev("palavra", "começou", "a"), None);
    }

    #[test]
    fn tem_bigrama_detecta_palavras_cadastradas() {
        assert!(tem_bigrama("molho"));
        assert!(tem_bigrama("colher"));
        assert!(tem_bigrama("bola"));
        assert!(!tem_bigrama("casa"));
        assert!(!tem_bigrama("palavra_inexistente"));
    }
}
