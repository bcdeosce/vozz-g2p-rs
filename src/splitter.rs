//! Divisão de texto em sentenças, respeitando abreviações, números decimais,
//! reticências, aspas e parênteses.
//!
//! Porte fiel de `splitter.js` do Vozz.

use once_cell::sync::Lazy;
use std::collections::HashSet;

/// Abreviações que não encerram sentença quando seguidas de ponto.
static ABREV: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "sr", "sra", "srta", "dr", "dra", "prof", "profa", "eng", "exmo",
        "av", "r", "pç", "ed", "apto", "ap", "pág", "pag", "fig", "obs",
        "ex", "etc", "cia", "ltda", "núm", "no", "tel", "cel", "jan", "fev",
        "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
        "seg", "ter", "qua", "qui", "sex", "sáb", "dom", "vs", "p", "pp",
    ]
    .into_iter()
    .collect()
});

/// Caracteres que encerram sentença.
fn eh_pontuacao_final(c: char) -> bool {
    matches!(c, '.' | '!' | '?' | '…')
}

/// Caracteres de fechamento absorvidos após a pontuação final.
fn eh_fechamento(c: char) -> bool {
    matches!(c, '"' | '\'' | ')' | ']' | '}' | '”' | '»' | '…')
}

/// Divide um texto completo em sentenças.
pub fn dividir_em_sentencas(texto: &str) -> Vec<String> {
    let normalizado = texto.replace("\r\n", "\n");
    if normalizado.trim().is_empty() {
        return Vec::new();
    }

    let chars: Vec<char> = normalizado.chars().collect();
    let mut sentencas: Vec<String> = Vec::new();
    let mut inicio: usize = 0;
    let mut i: usize = 0;

    while i < chars.len() {
        let c = chars[i];

        // Quebra de parágrafo sempre encerra a sentença.
        if c == '\n' && i + 1 < chars.len() && chars[i + 1] == '\n' {
            let s: String = chars[inicio..i].iter().collect();
            let s = s.trim().to_string();
            if !s.is_empty() {
                sentencas.push(s);
            }
            inicio = i + 1;
            i += 2;
            continue;
        }

        if eh_pontuacao_final(c) {
            let mut fim = i;
            while fim + 1 < chars.len() && eh_pontuacao_final(chars[fim + 1]) {
                fim += 1;
            }
            while fim + 1 < chars.len() && eh_fechamento(chars[fim + 1]) {
                fim += 1;
            }

            let mut k = fim + 1;
            while k < chars.len() && chars[k].is_whitespace() {
                k += 1;
            }
            let char_seguinte = chars.get(k).copied();

            let espaco_depois = chars
                .get(fim + 1)
                .map_or(true, |c| c.is_whitespace());

            let mut j = i;
            while j > 0 && !chars[j - 1].is_whitespace() {
                j -= 1;
            }
            let palavra_ant: String = chars[j..i]
                .iter()
                .copied()
                .filter(|c| c.is_alphabetic() || c.is_ascii_digit())
                .collect::<String>()
                .to_lowercase();

            let eh_abreviacao = c == '.' && ABREV.contains(palavra_ant.as_str());
            let eh_decimal = c == '.'
                && i > 0
                && chars[i - 1].is_ascii_digit()
                && i + 1 < chars.len()
                && chars[i + 1].is_ascii_digit();
            let eh_sigla = c == '.'
                && palavra_ant.chars().count() == 1
                && palavra_ant
                    .chars()
                    .next()
                    .map_or(false, |ch| ch.is_alphabetic());
            let continua_minuscula = char_seguinte
                .map_or(false, |ch| ch.is_lowercase() || ch.is_ascii_digit());

            if !eh_abreviacao
                && !eh_decimal
                && !eh_sigla
                && espaco_depois
                && !continua_minuscula
            {
                let s: String = chars[inicio..=fim].iter().collect();
                let s = s.trim().to_string();
                if !s.is_empty() {
                    sentencas.push(s);
                }
                inicio = fim + 1;
            }
            i = fim + 1;
            continue;
        }
        i += 1;
    }

    let resto: String = chars[inicio..].iter().collect();
    let resto = resto.trim().to_string();
    if !resto.is_empty() {
        sentencas.push(resto);
    }

    sentencas
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn sentenca_simples() {
        let r = dividir_em_sentencas("Olá mundo. Como vai?");
        assert_eq!(r, vec!["Olá mundo.", "Como vai?"]);
    }

    #[test]
    fn ponto_e_virgula_nao_quebra() {
        let r = dividir_em_sentencas("Primeiro; segundo. Terceiro!");
        assert_eq!(r, vec!["Primeiro; segundo.", "Terceiro!"]);
    }

    #[test]
    fn abreviacao_nao_quebra() {
        let r = dividir_em_sentencas("O Sr. Silva chegou. Bem-vindo!");
        assert_eq!(r, vec!["O Sr. Silva chegou.", "Bem-vindo!"]);
    }

    #[test]
    fn decimal_nao_quebra() {
        let r = dividir_em_sentencas("O valor é 3.14 reais. Entendeu?");
        assert_eq!(r, vec!["O valor é 3.14 reais.", "Entendeu?"]);
    }

    #[test]
    fn quebra_de_paragrafo() {
        let r = dividir_em_sentencas("Primeiro parágrafo.\n\nSegundo parágrafo.");
        assert_eq!(r, vec!["Primeiro parágrafo.", "Segundo parágrafo."]);
    }

    #[test]
    fn reticencias() {
        let r = dividir_em_sentencas("Ele pensou... E foi.");
        assert_eq!(r, vec!["Ele pensou...", "E foi."]);
    }

    #[test]
    fn texto_vazio() {
        assert!(dividir_em_sentencas("").is_empty());
        assert!(dividir_em_sentencas("   \n\n  ").is_empty());
    }

    #[test]
    fn sem_pontuacao_final() {
        let r = dividir_em_sentencas("Uma frase sem ponto final");
        assert_eq!(r, vec!["Uma frase sem ponto final"]);
    }

    #[test]
    fn sigla_uma_letra() {
        let r = dividir_em_sentencas("A. Silva chegou.");
        assert_eq!(r, vec!["A. Silva chegou."]);
    }

    #[test]
    fn ponto_seguido_de_minuscula_nao_quebra() {
        let r = dividir_em_sentencas("Ele disse: Oi! e saiu.");
        assert_eq!(r, vec!["Ele disse: Oi! e saiu."]);
    }

    #[test]
    fn multiplas_sentencas() {
        let r = dividir_em_sentencas("Primeira. Segunda! Terceira?");
        assert_eq!(r, vec!["Primeira.", "Segunda!", "Terceira?"]);
    }

    #[test]
    fn aspas_absorvidas() {
        let r = dividir_em_sentencas("Ele disse \"Olá.\" E saiu.");
        assert_eq!(r, vec!["Ele disse \"Olá.\"", "E saiu."]);
    }
}