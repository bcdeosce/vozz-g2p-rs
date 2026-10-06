//! Regras de trema (Acordo Ortográfico de 1990).
//!
//! O trema (¨) marcava o `u` átono e pronunciado após `q`/`g` antes de
//! vogal anterior (`e`, `i`, `é`, `ê`, `í`). O Acordo de 1990 removeu a
//! grafia, mas a pronúncia continua.
//!
//! Este módulo responde à pergunta:
//!
//! > dado "cinquenta" (sem trema), o `u` é /kw/ ou /k/?
//!
//! Estratégia:
//!
//! 1. Reintroduzir trema hipotético: `qu` antes de e/i/é/ê/í → `qü`.
//! 2. Se a forma está em `palavras_com_trema` → pronunciado.
//! 3. Se contém um `radical` conhecido → pronunciado.
//! 4. Caso contrário → deixa o chamador decidir (léxico ou regra padrão).
//!
//! O módulo NÃO decide o som final. Só diz se o `u` é pronunciado.
//! O chamador (`segmentar`) traduz isso em `kw`/`gw` vs `k`/`g`.

use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
struct ArquivoTrema {
    palavras_com_trema: Vec<String>,
    radicais: std::collections::HashMap<String, u32>,
    #[allow(dead_code)]
    regras_base: Vec<RegraBase>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct RegraBase {
    padrao: String,
    som: String,
    freq: u32,
}

static ARQUIVO: Lazy<ArquivoTrema> = Lazy::new(|| {
    let json = include_str!("../data/regras_trema.json");
    serde_json::from_str(json).expect("regras_trema.json inválido")
});

/// Conjunto de palavras com trema (lookup O(1)).
static PALAVRAS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    ARQUIVO
        .palavras_com_trema
        .iter()
        .map(|s| s.as_str())
        .collect()
});

/// Radicais ordenados por comprimento decrescente — os maiores primeiro
/// para casar antes dos menores e evitar que um radical curto capture
/// uma palavra que pertence a um radical mais específico.
static RADICAIS: Lazy<Vec<&'static str>> = Lazy::new(|| {
    let mut rads: Vec<&'static str> =
        ARQUIVO.radicais.keys().map(|s| s.as_str()).collect();
    rads.sort_by_key(|r| std::cmp::Reverse(r.len()));
    rads
});

/// Reintroduz trema hipotético em `qu`/`gu` antes de vogal anterior.
fn com_trema_hipotetico(palavra: &str) -> String {
    let caracteres: Vec<char> = palavra.chars().collect();
    let mut saida = String::with_capacity(palavra.len() + 4);
    let mut indice = 0;

    while indice < caracteres.len() {
        let caractere = caracteres[indice];

        if (caractere == 'q' || caractere == 'g')
            && indice + 2 < caracteres.len()
            && caracteres[indice + 1] == 'u'
        {
            let proximo = caracteres[indice + 2];
            if "eiéêí".contains(proximo) {
                saida.push(caractere);
                saida.push('ü');
                indice += 2;
                continue;
            }
        }

        saida.push(caractere);
        indice += 1;
    }

    saida
}

/// Diz se a palavra tem `u` pronunciado em `qu`/`gu` antes de vogal anterior.
pub fn u_pronunciado(palavra: &str) -> bool {
    if !palavra.contains('q') && !palavra.contains('g') {
        return false;
    }

    let hipotese = com_trema_hipotetico(palavra);

    if hipotese == palavra {
        return false;
    }

    if PALAVRAS.contains(hipotese.as_str()) {
        return true;
    }

    RADICAIS.iter().any(|radical| hipotese.contains(radical))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn com_trema_hipotetico_converte_qu_antes_de_e() {
        assert_eq!(com_trema_hipotetico("cinquenta"), "cinqüenta");
        assert_eq!(com_trema_hipotetico("quente"), "qüente");
    }

    #[test]
    fn com_trema_hipotetico_converte_antes_de_i() {
        assert_eq!(com_trema_hipotetico("aquilo"), "aqüilo");
        assert_eq!(com_trema_hipotetico("seguinte"), "següinte");
    }

    #[test]
    fn com_trema_hipotetico_nao_toca_antes_de_a_o() {
        assert_eq!(com_trema_hipotetico("quatro"), "quatro");
        assert_eq!(com_trema_hipotetico("quando"), "quando");
    }

    #[test]
    fn cinquenta_tem_u_pronunciado() {
        assert!(u_pronunciado("cinquenta"));
    }

    #[test]
    fn tranquilo_tem_u_pronunciado() {
        assert!(u_pronunciado("tranquilo"));
    }

    #[test]
    fn linguica_tem_u_pronunciado() {
        assert!(u_pronunciado("linguiça"));
    }

    #[test]
    fn quente_nao_tem_u_pronunciado() {
        assert!(!u_pronunciado("quente"));
    }

    #[test]
    fn querido_nao_tem_u_pronunciado() {
        assert!(!u_pronunciado("querido"));
    }

    #[test]
    fn guerra_nao_tem_u_pronunciado() {
        assert!(!u_pronunciado("guerra"));
    }

    #[test]
    fn seguinte_nao_tem_u_pronunciado() {
        assert!(!u_pronunciado("seguinte"));
    }

    #[test]
    fn quatro_nao_e_caso_de_trema() {
        assert!(!u_pronunciado("quatro"));
    }
}
