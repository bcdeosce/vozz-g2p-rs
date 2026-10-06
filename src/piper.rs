//! Conversão IPA → tokens do Piper (pt-BR).
//!
//! Porte fiel de `ipa_to_piper.py`. Aplica:
//!   1. remoção de separadores visuais (`_`, ZWJ, ZWNJ);
//!   2. NFD;
//!   3. mapa pt-BR `c → k`;
//!   4. filtro de flags `(xx)`;
//!   5. pontuação preservada como token individual;
//!   6. colapso de espaços duplicados.

use unicode_normalization::UnicodeNormalization;

const PUNCT: &[char] = &['.', ',', '!', '?', ':', ';', '…'];

/// Converte uma string IPA para a lista de tokens do Piper (pt-BR).
pub fn ipa_para_piper(ipa: &str) -> Vec<String> {
    // 1. Separadores visuais.
    let limpo: String = ipa
        .replace('_', "")
        .replace('\u{200D}', "")
        .replace('\u{200C}', "");

    // 2 + 3. NFD e mapa pt-BR (`c → k`).
    let mut mapeado: Vec<char> = Vec::with_capacity(limpo.len());
    for ch in limpo.nfd() {
        if ch == 'c' {
            mapeado.push('k');
        } else {
            mapeado.push(ch);
        }
    }

    // 4. Filtro de flags `(xx)`.
    let mut filtrado: Vec<char> = Vec::with_capacity(mapeado.len());
    let mut em_flag = false;
    for ch in mapeado {
        if em_flag {
            if ch == ')' {
                em_flag = false;
            }
        } else if ch == '(' {
            em_flag = true;
        } else {
            filtrado.push(ch);
        }
    }

    // 5. Pontuação vira token individual, cercada de espaços.
    let mut bruto: Vec<String> = Vec::with_capacity(filtrado.len() + 8);
    for ch in filtrado {
        if ch == '\n' {
            bruto.push(",".to_string());
            bruto.push(" ".to_string());
        } else if PUNCT.contains(&ch) {
            if bruto.last().map_or(false, |s| s != " ") {
                bruto.push(" ".to_string());
            }
            bruto.push(ch.to_string());
            bruto.push(" ".to_string());
        } else {
            bruto.push(ch.to_string());
        }
    }

    // 6. Colapsa espaços duplicados.
    let mut saida: Vec<String> = Vec::with_capacity(bruto.len());
    for token in bruto {
        if token == " " && saida.last().map_or(false, |s| s == " ") {
            continue;
        }
        saida.push(token);
    }

    saida
}

/// Versão de conveniência: junta os tokens em string (formato Piper textual).
pub fn ipa_para_piper_str(ipa: &str) -> String {
    ipa_para_piper(ipa).join("")
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn converte_simples() {
        let r = ipa_para_piper("kˈazæ");
        assert_eq!(r, vec!["k", "ˈ", "a", "z", "æ"]);
    }

    #[test]
    fn pontuacao_vira_token() {
        let r = ipa_para_piper("kˈazæ.");
        assert_eq!(r.last().unwrap(), ".");
        assert!(r.contains(&" ".to_string()));
    }

    #[test]
    fn mapeia_c_para_k() {
        let r = ipa_para_piper("casa");
        assert!(r.contains(&"k".to_string()));
        assert!(!r.contains(&"c".to_string()));
    }

    #[test]
    fn remove_flag_idioma() {
        let r = ipa_para_piper("(en)hello");
        assert!(!r.contains(&"(".to_string()));
        assert!(!r.contains(&")".to_string()));
    }

    #[test]
    fn newline_vira_virgula_espaco() {
        let r = ipa_para_piper("a\nb");
        let pos = r.iter().position(|s| s == ",").unwrap();
        assert_eq!(r[pos + 1], " ");
    }
}
