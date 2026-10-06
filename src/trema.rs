//! Regras de trema (Acordo Ortográfico de 1990).

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
struct ArquivoTrema {
    palavras_com_trema: Vec<String>,
    radicais: std::collections::HashMap<String, u32>,
    #[allow(dead_code)] regras_base: Vec<RegraBase>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct RegraBase { padrao: String, som: String, freq: u32 }

static ARQUIVO: Lazy<ArquivoTrema> = Lazy::new(|| {
    let json = include_str!("../data/regras_trema.json");
    serde_json::from_str(json).expect("regras_trema.json inválido")
});

static PALAVRAS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    ARQUIVO.palavras_com_trema.iter().map(|s| s.as_str()).collect()
});

static RADICAIS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut r: Vec<&'static str> = ARQUIVO.radicais.keys().map(|s| s.as_str()).collect();
    r.sort_by_key(|r| std::cmp::Reverse(r.len()));
    r
});

fn com_trema_hipotetico(palavra: &str) -> String {
    let chars: Vec<char> = palavra.chars().collect();
    let mut saida = String::with_capacity(palavra.len() + 4);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c == 'q' || c == 'g') && i + 2 < chars.len() && chars[i + 1] == 'u' {
            let prox = chars[i + 2];
            if "eiéêí".contains(prox) {
                saida.push(c); saida.push('ü'); i += 2; continue;
            }
        }
        saida.push(c); i += 1;
    }
    saida
}

pub fn u_pronunciado(palavra: &str) -> bool {
    if !palavra.contains('q') && !palavra.contains('g') { return false; }
    let hip = com_trema_hipotetico(palavra);
    if hip == palavra { return false; }
    if PALAVRAS.contains(hip.as_str()) { return true; }
    RADICAIS.iter().any(|r| hip.contains(r))
}

#[cfg(test)]
mod testes {
    use super::*;
    #[test] fn cinquenta_tem_u() { assert!(u_pronunciado("cinquenta")); }
    #[test] fn tranquilo_tem_u() { assert!(u_pronunciado("tranquilo")); }
    #[test] fn quente_nao_tem_u() { assert!(!u_pronunciado("quente")); }
    #[test] fn guerra_nao_tem_u() { assert!(!u_pronunciado("guerra")); }
    #[test] fn quatro_nao_tem_u() { assert!(!u_pronunciado("quatro")); }
}
