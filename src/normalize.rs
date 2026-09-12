//! Normalização de texto pt-BR.
//!
//! Porte de `normalize.js` do Vozz.
//!
//! Transforma números, datas, horas, moedas, abreviações e símbolos em
//! palavras antes da fonemização.
//!
//! Diferenças do original:
//!
//! - O `regex` do Rust não suporta lookahead/lookbehind. O padrão
//!   `(?!\s*[:h])` do JS foi reescrito como um grupo opcional `(\s*[:h])?`
//!   que, quando casa, devolve o match original sem substituição.
//!
//! - A função pública `precisa_normalizar` permite que o chamador pule
//!   o trabalho pesado de normalização quando a palavra é puramente
//!   alfabética e não é uma abreviação conhecida.
//!
//! - A função pública `eh_sigla_sem_vogal` detecta siglas em minúsculas
//!   (`ldl`, `ngf`, `cpk`) e interjeições sem vogal (`pst`, `shh`, `hm`),
//!   que o G2P não consegue fonemizar corretamente. Essas palavras devem
//!   ser filtradas ou resolvidas pelo léxico externo.

use crate::numbers::{
    ano_por_extenso, decimal_por_extenso, inteiro_por_extenso, ordinal_por_extenso,
    soletrar_digitos, OpcoesExtenso,
};
use once_cell::sync::Lazy;
use regex::{Captures, Regex, RegexBuilder};
use std::collections::{HashMap, HashSet};
use unicode_normalization::UnicodeNormalization;

const MESES: [&str; 12] = [
    "janeiro", "fevereiro", "março", "abril", "maio", "junho",
    "julho", "agosto", "setembro", "outubro", "novembro", "dezembro",
];

/// Vogais do português, com e sem diacríticos. Usado por `eh_sigla_sem_vogal`.
const VOGAIS_PT: &str = "aeiouáéíóúâêôãõàü";

static ABREVIACOES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut mapa = HashMap::new();
    mapa.insert("sr.", "senhor");
    mapa.insert("sr", "senhor");
    mapa.insert("sra.", "senhora");
    mapa.insert("sra", "senhora");
    mapa.insert("srta.", "senhorita");
    mapa.insert("dr.", "doutor");
    mapa.insert("dr", "doutor");
    mapa.insert("dra.", "doutora");
    mapa.insert("dra", "doutora");
    mapa.insert("prof.", "professor");
    mapa.insert("prof", "professor");
    mapa.insert("profa.", "professora");
    mapa.insert("profa", "professora");
    mapa.insert("eng.", "engenheiro");
    mapa.insert("av.", "avenida");
    mapa.insert("r.", "rua");
    mapa.insert("pç.", "praça");
    mapa.insert("ed.", "edifício");
    mapa.insert("apto.", "apartamento");
    mapa.insert("ap.", "apartamento");
    mapa.insert("pág.", "página");
    mapa.insert("pag.", "página");
    mapa.insert("págs.", "páginas");
    mapa.insert("fig.", "figura");
    mapa.insert("obs.", "observação");
    mapa.insert("ex.", "exemplo");
    mapa.insert("etc.", "etcétera");
    mapa.insert("etc", "etcétera");
    mapa.insert("cia.", "companhia");
    mapa.insert("ltda.", "limitada");
    mapa.insert("s.a.", "sociedade anônima");
    mapa.insert("núm.", "número");
    mapa.insert("nº", "número");
    mapa.insert("n°", "número");
    mapa.insert("no.", "número");
    mapa.insert("tel.", "telefone");
    mapa.insert("cel.", "celular");
    mapa.insert("kg", "quilogramas");
    mapa.insert("km", "quilômetros");
    mapa.insert("km/h", "quilômetros por hora");
    mapa.insert("cm", "centímetros");
    mapa.insert("mm", "milímetros");
    mapa.insert("ml", "mililitros");
    mapa.insert("mg", "miligramas");
    mapa.insert("gb", "gigabytes");
    mapa.insert("mb", "megabytes");
    mapa.insert("kb", "kilobytes");
    mapa.insert("tb", "terabytes");
    mapa
});

static SIMBOLOS: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    vec![
        ("%", " por cento "),
        ("&", " e "),
        ("@", " arroba "),
        ("+", " mais "),
        ("=", " igual a "),
        ("€", " euros "),
        ("£", " libras "),
        ("©", " copyright "),
        ("®", " marca registrada "),
        ("°c", " graus celsius "),
        ("°f", " graus fahrenheit "),
        ("º", " graus "),
        ("#", " cerquilha "),
        ("/", " barra "),
        ("\\", " barra invertida "),
        ("*", " asterisco "),
        ("_", " "),
        ("~", " "),
        ("^", " "),
        ("|", " "),
    ]
});

static SIGLAS_PALAVRA: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "onu", "otan", "fifa", "ibama", "inss", "cpf", "cep", "brics", "mercosul",
        "petrobras", "embraer", "unesco", "unicef", "senai", "sesc", "usp", "puc",
        "enem", "sus", "detran", "procon", "ipva", "fies", "prouni", "sisu",
    ]
    .into_iter()
    .collect()
});

fn letra_falada(c: char) -> Option<&'static str> {
    match c.to_ascii_lowercase() {
        'a' => Some("á"),
        'b' => Some("bê"),
        'c' => Some("cê"),
        'd' => Some("dê"),
        'e' => Some("é"),
        'f' => Some("éfe"),
        'g' => Some("gê"),
        'h' => Some("agá"),
        'i' => Some("i"),
        'j' => Some("jota"),
        'k' => Some("cá"),
        'l' => Some("éle"),
        'm' => Some("ême"),
        'n' => Some("êne"),
        'o' => Some("ó"),
        'p' => Some("pê"),
        'q' => Some("quê"),
        'r' => Some("érre"),
        's' => Some("ésse"),
        't' => Some("tê"),
        'u' => Some("u"),
        'v' => Some("vê"),
        'w' => Some("dábliu"),
        'x' => Some("xis"),
        'y' => Some("ípsilon"),
        'z' => Some("zê"),
        _ => None,
    }
}

/// Soletra uma sigla letra a letra.
pub fn soletrar_sigla(sigla: &str) -> String {
    let mut partes: Vec<String> = Vec::new();
    for c in sigla.chars() {
        match letra_falada(c) {
            Some(p) => partes.push(p.to_string()),
            None => partes.push(c.to_string()),
        }
    }
    partes.join(" ")
}

/// Diz se a palavra é uma sigla em minúsculas (sem vogais) ou uma
/// interjeição sem vogal.
///
/// O G2P não consegue fonemizar palavras sem vogais — ele produz algo
/// bizarro como `ldl` → `ˈwdw`. Essas palavras devem ser:
///
/// - filtradas da comparação (o `compare_vozz.py` faz isso);
/// - resolvidas pelo léxico externo (`lexico_espeak.json`), que tem a
///   pronúncia soletrada correta.
///
/// Regras:
///
/// - 2 a 6 caracteres (siglas curtas ou interjeições).
/// - Todos os caracteres são letras (sem dígitos, símbolos ou hífen).
/// - Nenhum caractere é vogal do português.
///
/// Exemplos:
///
/// | Palavra | Detectada? | Por quê |
/// |---------|------------|---------|
/// | `ldl`   | sim        | 3 letras, sem vogal |
/// | `ngf`   | sim        | 3 letras, sem vogal |
/// | `cpk`   | sim        | 3 letras, sem vogal |
/// | `lh`    | sim        | dígrafo, sem vogal |
/// | `pst`   | sim        | interjeição, sem vogal |
/// | `shh`   | sim        | interjeição, sem vogal |
/// | `hm`    | sim        | interjeição, sem vogal |
/// | `bem`   | não        | tem `e` |
/// | `sem`   | não        | tem `e` |
/// | `dom`   | não        | tem `o` |
/// | `casa`  | não        | tem `a` |
/// | `a`     | não        | 1 letra |
/// | `abcdefg` | não      | 7 letras |
/// | `n1`    | não        | tem dígito |
pub fn eh_sigla_sem_vogal(palavra: &str) -> bool {
    let p = palavra.to_lowercase();
    let n = p.chars().count();
    if !(2..=6).contains(&n) {
        return false;
    }
    if !p.chars().all(|c| c.is_alphabetic()) {
        return false;
    }
    !p.chars().any(|c| VOGAIS_PT.contains(c))
}

/// Diz se uma palavra precisa passar pelo normalizador.
///
/// Retorna `false` para palavras puramente alfabéticas que não são
/// abreviações conhecidas e não são siglas sem vogal — nesses casos,
/// o `normalizar` não faria nenhuma substituição e pode ser pulado.
pub fn precisa_normalizar(palavra: &str) -> bool {
    if palavra.chars().any(|c| !c.is_alphabetic()) {
        return true;
    }

    let chave = palavra.to_lowercase();
    if ABREVIACOES.contains_key(chave.as_str()) {
        return true;
    }

    eh_sigla_sem_vogal(palavra)
}

#[derive(Clone, Copy)]
pub struct OpcoesNormalizar {
    pub expandir_numeros: bool,
    pub expandir_siglas: bool,
}

impl Default for OpcoesNormalizar {
    fn default() -> Self {
        Self { expandir_numeros: true, expandir_siglas: true }
    }
}

// ---------------------------------------------------------------------------
// Regexes compiladas uma única vez
// ---------------------------------------------------------------------------

static RE_ASPAS_SIMPLES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\u{2018}\u{2019}\u{02BC}]").unwrap()
});
static RE_ASPAS_DUPLAS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\u{201C}\u{201D}\u{00AB}\u{00BB}]").unwrap()
});
static RE_TRACOS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\u{2013}\u{2014}\u{2212}]").unwrap()
});
static RE_RETICENCIAS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\u{2026}").unwrap()
});
static RE_ESPACOS_ESPECIAIS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\t\f\u{000B}\u{00A0}\u{200B}]").unwrap()
});
static RE_URL: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"https?://(?:www\.)?([^\s/]+)\S*")
        .case_insensitive(true)
        .build()
        .unwrap()
});
static RE_EMAIL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b([\w.+-]+)@([\w.-]+)\b").unwrap()
});
static RE_HORA_COMPLETA: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"\b(\d{1,2})\s*[h:]\s*(\d{2})\b(\s*[:h])?")
        .case_insensitive(true)
        .build()
        .unwrap()
});
static RE_HORA_SIMPLES: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"\b(\d{1,2})\s*h\b")
        .case_insensitive(true)
        .build()
        .unwrap()
});
static RE_DATA: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(\d{1,2})[/-](\d{1,2})[/-](\d{2,4})\b").unwrap()
});
static RE_MOEDA_BRL: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"R\$\s?([\d.]+)(?:,(\d{1,2}))?")
        .case_insensitive(true)
        .build()
        .unwrap()
});
static RE_MOEDA_USD: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"US\$\s?([\d.]+)(?:,(\d{1,2}))?")
        .case_insensitive(true)
        .build()
        .unwrap()
});
static RE_PERCENTUAL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\d+(?:,\d+)?)\s*%").unwrap()
});
static RE_TEMPERATURA: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"(\d+)\s*°\s*C\b")
        .case_insensitive(true)
        .build()
        .unwrap()
});
static RE_ORDINAL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(\d{1,3})\s*([ºª°])").unwrap()
});
static RE_TELEFONE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\(?\b(\d{2})\)?\s?9?\d{4}-\d{4}\b").unwrap()
});
static RE_DECIMAL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(\d{1,3}(?:\.\d{3})+|\d+),(\d+)\b").unwrap()
});
static RE_MILHAR: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{1,3}(?:\.\d{3})+\b").unwrap()
});
static RE_INTEIRO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d+").unwrap()
});
static RE_ABREVIACAO: Lazy<Regex> = Lazy::new(|| {
    RegexBuilder::new(r"\b([a-z\u{00E0}-\u{00FF}]{1,6}\.?)")
        .case_insensitive(true)
        .build()
        .unwrap()
});
static RE_SIGLA: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Z\u{00C0}-\u{00DE}]{2,6}\b").unwrap()
});
static RE_ESPACOS_MULTIPLOS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r" {2,}").unwrap()
});

// ---------------------------------------------------------------------------
// Função principal
// ---------------------------------------------------------------------------

pub fn normalizar(texto: &str, opcoes: OpcoesNormalizar) -> String {
    let mut t: String = texto.nfc().collect();

    t = RE_ASPAS_SIMPLES.replace_all(&t, "'").into_owned();
    t = RE_ASPAS_DUPLAS.replace_all(&t, "\"").into_owned();
    t = RE_TRACOS.replace_all(&t, "—").into_owned();
    t = RE_RETICENCIAS.replace_all(&t, "...").into_owned();
    t = RE_ESPACOS_ESPECIAIS.replace_all(&t, " ").into_owned();

    t = RE_URL
        .replace_all(&t, |caps: &Captures| {
            let dom = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            format!(" {} ", dom.replace('.', " ponto "))
        })
        .into_owned();

    t = RE_EMAIL
        .replace_all(&t, |caps: &Captures| {
            let usuario = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let dominio = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            format!(
                " {} arroba {} ",
                usuario.replace('.', " ponto "),
                dominio.replace('.', " ponto ")
            )
        })
        .into_owned();

    t = RE_HORA_COMPLETA
        .replace_all(&t, |caps: &Captures| {
            if caps.get(3).is_some() {
                return caps.get(0).unwrap().as_str().to_string();
            }
            let hh: i64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let mm: i64 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);

            let op_fem = OpcoesExtenso { feminino: true };
            let unidade_hora = if hh == 1 { "hora" } else { "horas" };
            let hora = format!("{} {}", inteiro_por_extenso(hh, op_fem), unidade_hora);

            if mm == 0 {
                return format!(" {} ", hora);
            }
            let unidade_minuto = if mm == 1 { "minuto" } else { "minutos" };
            let minuto = format!(
                "{} {}",
                inteiro_por_extenso(mm, OpcoesExtenso::default()),
                unidade_minuto
            );
            format!(" {} e {} ", hora, minuto)
        })
        .into_owned();

    t = RE_HORA_SIMPLES
        .replace_all(&t, |caps: &Captures| {
            let hh: i64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let unidade = if hh == 1 { "hora" } else { "horas" };
            let op_fem = OpcoesExtenso { feminino: true };
            format!(" {} {} ", inteiro_por_extenso(hh, op_fem), unidade)
        })
        .into_owned();

    t = RE_DATA
        .replace_all(&t, |caps: &Captures| {
            let dia: u32 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let mes: u32 = caps.get(2).unwrap().as_str().parse().unwrap_or(0);
            let ano_txt = caps.get(3).unwrap().as_str();

            if !(1..=12).contains(&mes) || !(1..=31).contains(&dia) {
                return caps.get(0).unwrap().as_str().to_string();
            }

            let dia_txt = if dia == 1 {
                "primeiro".to_string()
            } else {
                inteiro_por_extenso(dia as i64, OpcoesExtenso::default())
            };
            let ano_num: i64 = ano_txt.parse().unwrap_or(0);
            format!(
                " {} de {} de {} ",
                dia_txt,
                MESES[(mes - 1) as usize],
                ano_por_extenso(ano_num)
            )
        })
        .into_owned();

    t = RE_MOEDA_BRL
        .replace_all(&t, |caps: &Captures| {
            let int_txt = caps.get(1).unwrap().as_str().replace('.', "");
            let n: i64 = int_txt.parse().unwrap_or(0);
            let unidade = if n == 1 { "real" } else { "reais" };
            let mut out = format!(
                " {} {} ",
                inteiro_por_extenso(n, OpcoesExtenso::default()),
                unidade
            );

            if let Some(cent_match) = caps.get(2) {
                let cent_txt = cent_match.as_str();
                let cent_padded = format!("{:0<2}", cent_txt);
                let cent: i64 = cent_padded.parse().unwrap_or(0);
                if cent > 0 {
                    let unidade_cent = if cent == 1 { "centavo" } else { "centavos" };
                    out = format!(
                        "{} e {} {}",
                        out.trim_end(),
                        inteiro_por_extenso(cent, OpcoesExtenso::default()),
                        unidade_cent
                    );
                }
            }
            out
        })
        .into_owned();

    t = RE_MOEDA_USD
        .replace_all(&t, |caps: &Captures| {
            let int_txt = caps.get(1).unwrap().as_str().replace('.', "");
            let n: i64 = int_txt.parse().unwrap_or(0);
            format!(" {} dólares ", inteiro_por_extenso(n, OpcoesExtenso::default()))
        })
        .into_owned();

    t = RE_PERCENTUAL
        .replace_all(&t, |caps: &Captures| {
            let n = caps.get(1).unwrap().as_str();
            match n.split_once(',') {
                Some((i, f)) => {
                    let inteiro: i64 = i.parse().unwrap_or(0);
                    format!(
                        " {} por cento ",
                        decimal_por_extenso(inteiro, f, OpcoesExtenso::default())
                    )
                }
                None => {
                    let inteiro: i64 = n.parse().unwrap_or(0);
                    format!(
                        " {} por cento ",
                        inteiro_por_extenso(inteiro, OpcoesExtenso::default())
                    )
                }
            }
        })
        .into_owned();

    t = RE_TEMPERATURA
        .replace_all(&t, |caps: &Captures| {
            let n: i64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            format!(
                " {} graus celsius ",
                inteiro_por_extenso(n, OpcoesExtenso::default())
            )
        })
        .into_owned();

    t = RE_ORDINAL
        .replace_all(&t, |caps: &Captures| {
            let n: i64 = caps.get(1).unwrap().as_str().parse().unwrap_or(0);
            let marca = caps.get(2).unwrap().as_str();
            let feminino = marca == "ª";
            format!(" {} ", ordinal_por_extenso(n, OpcoesExtenso { feminino }))
        })
        .into_owned();

    t = RE_TELEFONE
        .replace_all(&t, |caps: &Captures| {
            let bruto = caps.get(0).unwrap().as_str();
            let limpo: String = bruto.chars().filter(|c| *c != '(' && *c != ')').collect();
            format!(" {} ", soletrar_digitos(&limpo))
        })
        .into_owned();

    if opcoes.expandir_numeros {
        t = RE_DECIMAL
            .replace_all(&t, |caps: &Captures| {
                let i = caps.get(1).unwrap().as_str().replace('.', "");
                let f = caps.get(2).unwrap().as_str();
                let inteiro: i64 = i.parse().unwrap_or(0);
                format!(
                    " {} ",
                    decimal_por_extenso(inteiro, f, OpcoesExtenso::default())
                )
            })
            .into_owned();

        t = RE_MILHAR
            .replace_all(&t, |caps: &Captures| {
                let s = caps.get(0).unwrap().as_str().replace('.', "");
                let n: i64 = s.parse().unwrap_or(0);
                format!(" {} ", inteiro_por_extenso(n, OpcoesExtenso::default()))
            })
            .into_owned();

        t = RE_INTEIRO
            .replace_all(&t, |caps: &Captures| {
                let s = caps.get(0).unwrap().as_str();
                let n: i64 = s.parse().unwrap_or(0);
                format!(" {} ", inteiro_por_extenso(n, OpcoesExtenso::default()))
            })
            .into_owned();
    }

    t = RE_ABREVIACAO
        .replace_all(&t, |caps: &Captures| {
            let m = caps.get(0).unwrap().as_str();
            let chave = m.to_lowercase();
            match ABREVIACOES.get(chave.as_str()) {
                Some(v) => (*v).to_string(),
                None => m.to_string(),
            }
        })
        .into_owned();

    if opcoes.expandir_siglas {
        t = RE_SIGLA
            .replace_all(&t, |caps: &Captures| {
                let m = caps.get(0).unwrap().as_str();
                let baixa = m.to_lowercase();

                if SIGLAS_PALAVRA.contains(baixa.as_str()) {
                    return baixa;
                }

                let tem_vogal = m.chars().any(|c| "AEIOU".contains(c));
                let n_chars = m.chars().count();
                if tem_vogal && n_chars >= 4 {
                    let casa_padrao = m.chars().all(|c| {
                        c.is_ascii_uppercase() || ('\u{00C0}'..='\u{00DE}').contains(&c)
                    }) && m.chars().any(|c| "AEIOU".contains(c));
                    if !casa_padrao {
                        return baixa;
                    }
                }

                format!(" {} ", soletrar_sigla(m))
            })
            .into_owned();
    }

    for (simbolo, texto_substituto) in SIMBOLOS.iter() {
        t = t.replace(simbolo, texto_substituto);
    }

    t = RE_ESPACOS_MULTIPLOS.replace_all(&t, " ").into_owned();
    t.trim().to_string()
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod testes {
    use super::*;

    fn norm(texto: &str) -> String {
        normalizar(texto, OpcoesNormalizar::default())
    }

    // --- Aspas, traços, símbolos ---

    #[test]
    fn aspas_e_tracos() {
        assert_eq!(norm("olá \u{201C}mundo\u{201D}"), "olá \"mundo\"");
        assert_eq!(norm("ele \u{2018}disse\u{2019}"), "ele 'disse'");
        assert_eq!(norm("a\u{2013}b"), "a—b");
        assert_eq!(norm("fim\u{2026}"), "fim...");
    }

    #[test]
    fn simbolos() {
        assert_eq!(norm("a & b"), "a e b");
        assert_eq!(norm("x = 5"), "x igual a cinco");
    }

    // --- Números ---

    #[test]
    fn numero_simples() {
        assert_eq!(norm("42"), "quarenta e dois");
        assert_eq!(norm("100"), "cem");
        assert_eq!(norm("2025"), "dois mil e vinte e cinco");
    }

    #[test]
    fn decimal_com_virgula() {
        assert_eq!(norm("3,14"), "três vírgula quatorze");
    }

    #[test]
    fn milhar_com_ponto() {
        assert_eq!(norm("1.234"), "mil duzentos e trinta e quatro");
    }

    #[test]
    fn nao_expandir_numeros() {
        let o = OpcoesNormalizar { expandir_numeros: false, expandir_siglas: true };
        assert_eq!(normalizar("42", o), "42");
    }

    // --- Datas, horas, moedas ---

    #[test]
    fn data() {
        assert_eq!(
            norm("07/09/2025"),
            "sete de setembro de dois mil e vinte e cinco"
        );
    }

    #[test]
    fn hora_completa() {
        assert_eq!(norm("14:30"), "quatorze horas e trinta minutos");
        assert_eq!(norm("9h15"), "nove horas e quinze minutos");
    }

    #[test]
    fn hora_simples() {
        assert_eq!(norm("9h"), "nove horas");
    }

    #[test]
    fn moeda_brl() {
        assert_eq!(norm("R$ 10"), "dez reais");
        assert_eq!(norm("R$ 1"), "um real");
    }

    #[test]
    fn percentual() {
        assert_eq!(norm("50%"), "cinquenta por cento");
    }

    #[test]
    fn temperatura() {
        assert_eq!(norm("30°C"), "trinta graus celsius");
    }

    // --- Ordinais ---

    #[test]
    fn ordinal_masculino() {
        assert_eq!(norm("1º"), "primeiro");
        assert_eq!(norm("2º"), "segundo");
    }

    #[test]
    fn ordinal_feminino() {
        assert_eq!(norm("1ª"), "primeira");
        assert_eq!(norm("2ª"), "segunda");
    }

    // --- Abreviações e siglas ---

    #[test]
    fn abreviacao() {
        assert_eq!(norm("Sr. Silva"), "senhor Silva");
        assert_eq!(norm("Dra. Ana"), "doutora Ana");
        assert_eq!(norm("Dr. João"), "doutor João");
    }

    #[test]
    fn sigla_soletrada() {
        let r = norm("XYZ");
        assert!(r.contains("xis"), "esperava 'xis' em: {}", r);
        assert!(r.contains("ípsilon"), "esperava 'ípsilon' em: {}", r);
        assert!(r.contains("zê"), "esperava 'zê' em: {}", r);
    }

    #[test]
    fn sigla_como_palavra() {
        assert_eq!(norm("ONU"), "onu");
    }

    // --- precisa_normalizar ---

    #[test]
    fn precisa_normalizar_palavra_alfabetica() {
        assert!(!precisa_normalizar("casa"));
        assert!(!precisa_normalizar("banana"));
        assert!(!precisa_normalizar("exemplo"));
        assert!(!precisa_normalizar("Brasil"));
    }

    #[test]
    fn precisa_normalizar_abreviacoes() {
        assert!(precisa_normalizar("dr"));
        assert!(precisa_normalizar("sr"));
        assert!(precisa_normalizar("dra"));
        assert!(precisa_normalizar("prof"));
        assert!(precisa_normalizar("etc"));
        assert!(precisa_normalizar("kg"));
        assert!(precisa_normalizar("cm"));
    }

    #[test]
    fn precisa_normalizar_abreviacoes_case_insensitive() {
        assert!(precisa_normalizar("Dr"));
        assert!(precisa_normalizar("SR"));
        assert!(precisa_normalizar("Dra"));
        assert!(precisa_normalizar("PROF"));
    }

    #[test]
    fn precisa_normalizar_com_ponto() {
        assert!(precisa_normalizar("dr."));
        assert!(precisa_normalizar("sr."));
        assert!(precisa_normalizar("etc."));
        assert!(precisa_normalizar("casa."));
    }

    #[test]
    fn precisa_normalizar_com_digitos() {
        assert!(precisa_normalizar("100"));
        assert!(precisa_normalizar("100mg"));
        assert!(precisa_normalizar("3,14"));
        assert!(precisa_normalizar("2025"));
    }

    #[test]
    fn precisa_normalizar_com_simbolos() {
        assert!(precisa_normalizar("50%"));
        assert!(precisa_normalizar("R$"));
        assert!(precisa_normalizar("a&b"));
        assert!(precisa_normalizar("x=5"));
    }

    // --- eh_sigla_sem_vogal ---

    #[test]
    fn sigla_sem_vogal_detecta_ldl() {
        assert!(eh_sigla_sem_vogal("ldl"));
        assert!(eh_sigla_sem_vogal("LDL"));
    }

    #[test]
    fn sigla_sem_vogal_detecta_ngf() {
        assert!(eh_sigla_sem_vogal("ngf"));
        assert!(eh_sigla_sem_vogal("NGF"));
        assert!(eh_sigla_sem_vogal("vldl"));
        assert!(eh_sigla_sem_vogal("drc"));
    }

    #[test]
    fn sigla_sem_vogal_detecta_curtas() {
        for p in ["cpk", "mhc", "tnf", "kgf", "fff", "df", "drc", "vldl"] {
            assert!(eh_sigla_sem_vogal(p), "{}: esperava detectar", p);
        }
    }

    #[test]
    fn sigla_sem_vogal_detecta_digrafos() {
        for p in ["lh", "nh", "ch", "rr", "ss"] {
            assert!(
                eh_sigla_sem_vogal(p),
                "{}: esperava detectar (2 letras, sem vogal)", p
            );
        }
    }

    #[test]
    fn sigla_sem_vogal_detecta_interjeicoes() {
        for p in ["pst", "shh", "hm", "shhh", "mm", "nn", "psst"] {
            assert!(
                eh_sigla_sem_vogal(p),
                "{}: esperava detectar interjeição", p
            );
        }
    }

    #[test]
    fn sigla_sem_vogal_nao_detecta_palavras_com_vogal() {
        for p in [
            "bem", "sem", "dom", "casa", "banana", "sol", "mar", "paz",
            "xyz", "n1", "abc",
        ] {
            // "xyz" tem y mas não tem vogal PT: é detectada.
            // Vou testar separadamente.
            if p == "xyz" {
                assert!(
                    eh_sigla_sem_vogal(p),
                    "{}: xyz deve ser detectada (sem vogal PT)", p
                );
            } else {
                assert!(
                    !eh_sigla_sem_vogal(p),
                    "{}: não esperava detectar", p
                );
            }
        }
    }

    #[test]
    fn sigla_sem_vogal_nao_detecta_uma_letra() {
        for p in ["a", "e", "b", "c", "d", "x"] {
            assert!(
                !eh_sigla_sem_vogal(p),
                "{}: 1 letra não deve ser detectada", p
            );
        }
    }

    #[test]
    fn sigla_sem_vogal_nao_detecta_muito_longa() {
        // 7+ letras: não é sigla curta nem interjeição.
        for p in ["abcdefg", "bcdfghjkl", "mnbvcxz"] {
            assert!(
                !eh_sigla_sem_vogal(p),
                "{}: muito longa, não deve ser detectada", p
            );
        }
    }

    #[test]
    fn sigla_sem_vogal_nao_detecta_com_digito() {
        for p in ["n1", "4g", "3d", "a1b"] {
            assert!(
                !eh_sigla_sem_vogal(p),
                "{}: tem dígito, não deve ser detectada", p
            );
        }
    }

    #[test]
    fn sigla_sem_vogal_nao_detecta_com_acento() {
        // Vogais com acento também são vogais.
        for p in ["cá", "pé", "só", "gê", "vê"] {
            assert!(
                !eh_sigla_sem_vogal(p),
                "{}: tem vogal acentuada, não deve ser detectada", p
            );
        }
    }

    #[test]
    fn sigla_sem_vogal_detecta_www() {
        assert!(eh_sigla_sem_vogal("www"));
    }

    #[test]
    fn sigla_sem_vogal_case_insensitive() {
        assert_eq!(eh_sigla_sem_vogal("ldl"), eh_sigla_sem_vogal("LDL"));
        assert_eq!(eh_sigla_sem_vogal("ngf"), eh_sigla_sem_vogal("NGF"));
        assert_eq!(eh_sigla_sem_vogal("pst"), eh_sigla_sem_vogal("PST"));
    }

    #[test]
    fn precisa_normalizar_inclui_siglas_sem_vogal() {
        // Siglas sem vogais agora contam como "precisa normalizar",
        // porque o chamador pode querer descartá-las ou resolvê-las
        // pelo léxico externo.
        for p in ["ldl", "ngf", "cpk", "pst", "shh", "hm", "lh"] {
            assert!(
                precisa_normalizar(p),
                "{}: esperava precisar normalizar", p
            );
        }
    }

    #[test]
    fn precisa_normalizar_nao_inclui_palavras_comuns() {
        // Palavras com vogal continuam fora.
        for p in ["casa", "banana", "bem", "sem", "dom", "sol"] {
            assert!(
                !precisa_normalizar(p),
                "{}: não esperava precisar normalizar", p
            );
        }
    }
}
