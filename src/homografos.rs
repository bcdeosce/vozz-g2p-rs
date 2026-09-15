//! Desambiguador de homógrafos heterofônicos pt-BR.
//!
//! Reimplementação em Rust da arquitetura de classificação do
//! [Bifonia](https://github.com/TigreGotico/bifonia) (Apache 2.0),
//! adaptada para pt-BR.
//!
//! ## Ordem de decisão em `desambiguar`
//!
//! 1. Regra `single` → devolve o sentido fixo.
//! 2. Expressões fixas (com lookahead de 2 palavras) → sentido fixo.
//! 3. Naive Bayes sobre as 7 features (6 originais + `next_word_2`).

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

static RE_WORD: Lazy<Regex> = Lazy::new(|| Regex::new(r"\w+").unwrap());

/* ------------------------------------------------------------------ *
 * Tipos públicos
 * ------------------------------------------------------------------ */

/// Regra de desambiguação carregada de `homograph_rules_v2.json`.
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Regra {
    Single {
        sense: String,
        pos: String,
    },
    Multi {
        senses: Vec<String>,
        prior: HashMap<String, f64>,
        feat_counts: HashMap<String, HashMap<String, HashMap<String, usize>>>,
        feat_totals: HashMap<String, usize>,
    },
}

/// Resultado de uma desambiguação.
#[derive(Debug, Clone)]
pub struct Disambiguacao {
    pub sentido: String,
    #[allow(dead_code)]
    pub pos: String,
    #[allow(dead_code)]
    pub metodo: &'static str,
}

#[derive(Deserialize, Debug, Clone)]
struct EntradaSentido {
    ipa: String,
    #[serde(default)]
    #[allow(dead_code)]
    confianca: f64,
    #[serde(default)]
    #[allow(dead_code)]
    ocorrencias: usize,
}

/// Diagnóstico de carregamento do desambiguador.
#[derive(Serialize, Debug, Clone)]
pub struct DiagnosticoHomografos {
    pub n_regras: usize,
    pub regras_single: usize,
    pub regras_multi: usize,
    pub n_palavras_lexicon: usize,
    pub n_total_entradas_lexicon: usize,
    pub n_expressoes_fixas: usize,
    pub n_expressoes_com_next2: usize,
    pub caminho_regras: String,
    pub caminho_lexicon: String,
}

/// Desambiguador carregado em memória.
pub struct Homografos {
    regras: HashMap<String, Regra>,
    lexico: HashMap<String, HashMap<String, String>>,
    caminho_regras: String,
    caminho_lexicon: String,
}

/* ------------------------------------------------------------------ *
 * Expressões fixas contextuais
 * ------------------------------------------------------------------ */

/// Expressões fixas com até 2 palavras de lookahead.
///
/// Formato: `(palavra, prev, next, next_2, sense, pos)`.
static EXPRESSOES_FIXAS: &[(
    &str,
    Option<&str>,
    Option<&str>,
    Option<&str>,
    &str,
    &str,
)] = &[
    /* 1. `pelo` + palavra */
    ("pelo", None, Some("visto"),     None, "by_the", "ADP"),
    ("pelo", None, Some("menos"),     None, "by_the", "ADP"),
    ("pelo", None, Some("contrário"), None, "by_the", "ADP"),
    ("pelo", None, Some("contrario"), None, "by_the", "ADP"),
    ("pelo", None, Some("amor"),      None, "by_the", "ADP"),
    ("pelo", None, Some("jeito"),     None, "by_the", "ADP"),
    ("pelo", None, Some("mundo"),     None, "by_the", "ADP"),
    ("pelo", None, Some("fato"),      None, "by_the", "ADP"),
    ("pelo", None, Some("caminho"),   None, "by_the", "ADP"),
    ("pelo", None, Some("tempo"),     None, "by_the", "ADP"),
    ("pelo", None, Some("silêncio"),  None, "by_the", "ADP"),
    ("pelo", None, Some("silencio"),  None, "by_the", "ADP"),
    ("pelo", None, Some("que"),       None, "by_the", "ADP"),

    /* 2. `pela` + palavra */
    ("pela", None, Some("manhã"),    None, "by_the", "ADP"),
    ("pela", None, Some("manha"),    None, "by_the", "ADP"),
    ("pela", None, Some("tarde"),    None, "by_the", "ADP"),
    ("pela", None, Some("noite"),    None, "by_the", "ADP"),
    ("pela", None, Some("primeira"), None, "by_the", "ADP"),
    ("pela", None, Some("última"),   None, "by_the", "ADP"),
    ("pela", None, Some("ultima"),   None, "by_the", "ADP"),

    /* 3. Verbo 1sg com sujeito/advérbio */
    ("porto",    Some("sempre"), None, None, "carry",   "VERB"),
    ("porto",    Some("eu"),     None, None, "carry",   "VERB"),
    ("sopro",    Some("eu"),     None, None, "blow",    "VERB"),
    ("desapego", Some("eu"),     None, None, "let_go",  "VERB"),
    ("despojo",  Some("nunca"),  None, None, "strip",   "VERB"),
    ("acerto",   Some("nunca"),  None, None, "adjust",  "VERB"),
    ("solto",    Some("eu"),     None, None, "release", "VERB"),
    ("desaforo", Some("posso"),  None, None, "affront", "VERB"),
    ("congelo",  Some("quando"), None, None, "freeze",  "VERB"),

    /* 4. Verbo 1sg — next_word discrimina */
    ("torno",  None, Some("ao"), None, "turn", "VERB"),
    ("torno",  None, Some("à"),  None, "turn", "VERB"),
    ("porto",  None, Some("atitudes"), None, "carry", "VERB"),
    ("porto",  None, Some("verdades"), None, "carry", "VERB"),
    ("colher", None, Some("depoimentos"), None, "harvest", "VERB"),
    ("colher", None, Some("notas"),       None, "harvest", "VERB"),

    /* 5. Substantivos/adjetivos */
    ("cor",   None, Some("exata"), None, "colour", "NOUN"),
    ("cor",   None, Some("ideal"), None, "colour", "NOUN"),
    ("torre", None, Some("eólica"), None, "tower", "NOUN"),
    ("sede",  Some("nova"), None, None, "seat", "NOUN"),
    ("lobo",  Some("do"),   None, None, "wolf", "NOUN"),

    /* 6. Bigramas contextuais — discriminante em +2 */
    ("molho", None, Some("de"), Some("palha"),       "bundle", "NOUN"),
    ("molho", None, Some("de"), Some("especiarias"), "bundle", "NOUN"),
    ("molho", None, Some("de"), Some("cogumelos"),   "sauce",  "NOUN"),
    ("molho", None, Some("de"), Some("wasabi"),      "sauce",  "NOUN"),
    ("bola",  None, Some("de"), Some("carne"),   "loaf", "NOUN"),
    ("bola",  None, Some("de"), Some("futebol"), "ball", "NOUN"),
    ("polo",  None, Some("de"), Some("energia"), "hub", "NOUN"),
    ("polo",  None, Some("de"), Some("água"),    "hub", "NOUN"),
];

fn check_expressao_fixa(
    w: &str,
    prev: &str,
    nxt: &str,
    nxt2: &str,
) -> Option<(&'static str, &'static str)> {
    for (palavra, pl, pr, pr2, sense, pos) in EXPRESSOES_FIXAS {
        if *palavra != w {
            continue;
        }
        let ok_l = pl.map_or(true, |x| x == prev);
        let ok_r = pr.map_or(true, |x| x == nxt);
        let ok_r2 = pr2.map_or(true, |x| x == nxt2);
        if ok_l && ok_r && ok_r2 {
            return Some((sense, pos));
        }
    }
    None
}

/* ------------------------------------------------------------------ *
 * Classes gramaticais
 * ------------------------------------------------------------------ */

const ARTIGOS: &[&str] = &["o", "a", "os", "as", "um", "uma", "uns", "umas"];
const DEMONSTR: &[&str] = &[
    "este", "esta", "estes", "estas", "esse", "essa", "esses", "essas",
    "isso", "isto", "aquele", "aquela", "aqueles", "aquelas", "aquilo",
];
const PREPOSICOES: &[&str] = &[
    "de", "do", "da", "dos", "das", "em", "no", "na", "nos", "nas",
    "por", "pelo", "pela", "pelos", "pelas", "com", "sem", "para",
    "ao", "à", "aos", "às", "entre", "sobre", "sob", "contra",
    "desde", "até",
];
const PRONOMES: &[&str] = &[
    "eu", "tu", "ele", "ela", "nós", "vós", "você", "vocês", "eles", "elas",
    "me", "te", "lhe", "nos", "vos", "se", "mim", "ti", "si",
];
const CONJUNCOES: &[&str] = &[
    "e", "ou", "mas", "porém", "contudo", "todavia", "entretanto", "pois",
    "porque", "que", "se", "como", "quando", "onde", "embora", "caso",
];
const ADVERBIOS: &[&str] = &[
    "não", "sim", "muito", "mais", "menos", "também", "só", "já", "ainda",
    "sempre", "nunca", "aqui", "ali", "lá", "cá", "agora", "depois",
    "antes", "então", "assim", "bem", "mal", "tão", "quase",
];

pub fn classify_token(t: &str) -> &'static str {
    if t.is_empty() {
        return "NONE";
    }
    if ARTIGOS.contains(&t) { return "ART"; }
    if DEMONSTR.contains(&t) { return "DEM"; }
    if PREPOSICOES.contains(&t) { return "PREP"; }
    if PRONOMES.contains(&t) { return "PRON"; }
    if CONJUNCOES.contains(&t) { return "CONJ"; }
    if ADVERBIOS.contains(&t) { return "ADV"; }
    if t.len() > 3 && (t.ends_with("ar") || t.ends_with("er") || t.ends_with("ir")) {
        return "VERB_INF";
    }
    if t.len() > 4
        && (t.ends_with("ando") || t.ends_with("endo") || t.ends_with("indo"))
    {
        return "VERB_GER";
    }
    if t.len() > 4 && (t.ends_with("ado") || t.ends_with("ido")) {
        return "VERB_PART";
    }
    if t.len() > 6 && t.ends_with("mente") {
        return "ADV";
    }
    "OTHER"
}

/* ------------------------------------------------------------------ *
 * Features
 * ------------------------------------------------------------------ */

pub fn norm_word(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

fn tokenize(s: &str) -> Vec<String> {
    RE_WORD
        .find_iter(s)
        .map(|m| m.as_str().to_lowercase())
        .collect()
}

fn extract_features(
    sentence: &str,
    target: &str,
) -> Option<Vec<(&'static str, String)>> {
    let toks = tokenize(sentence);
    let tw = norm_word(target);
    let i = toks.iter().position(|t| *t == tw)?;

    let prev = if i > 0 { Some(toks[i - 1].as_str()) } else { None };
    let next = if i + 1 < toks.len() { Some(toks[i + 1].as_str()) } else { None };
    let next2 = if i + 2 < toks.len() { Some(toks[i + 2].as_str()) } else { None };

    let mut feats: Vec<(&'static str, String)> = Vec::with_capacity(7);
    feats.push(("prev_word", prev.unwrap_or("").to_string()));
    feats.push(("next_word", next.unwrap_or("").to_string()));
    feats.push(("next_word_2", next2.unwrap_or("").to_string()));
    feats.push(("prev_class", classify_token(prev.unwrap_or("")).to_string()));
    feats.push(("next_class", classify_token(next.unwrap_or("")).to_string()));
    feats.push(("is_first", if i == 0 { "true" } else { "false" }.to_string()));
    let pos_in_sent = if i == 0 {
        "first"
    } else if i == toks.len() - 1 {
        "last"
    } else {
        "middle"
    };
    feats.push(("pos_in_sent", pos_in_sent.to_string()));

    Some(feats)
}

/* ------------------------------------------------------------------ *
 * Impl
 * ------------------------------------------------------------------ */

impl Homografos {
    pub fn novo(
        regras: HashMap<String, Regra>,
        lexico: HashMap<String, HashMap<String, String>>,
    ) -> Self {
        Self {
            regras,
            lexico,
            caminho_regras: "<memoria>".to_string(),
            caminho_lexicon: "<memoria>".to_string(),
        }
    }

    pub fn from_paths(regras_path: &Path, lexicon_path: &Path) -> Result<Self> {
        let f_regras = File::open(regras_path)
            .map_err(|e| format!("abrindo {}: {}", regras_path.display(), e))?;
        let regras: HashMap<String, Regra> =
            serde_json::from_reader(BufReader::new(f_regras))
                .map_err(|e| format!("parseando regras JSON: {}", e))?;

        let f_lex = File::open(lexicon_path)
            .map_err(|e| format!("abrindo {}: {}", lexicon_path.display(), e))?;
        let lexico_completo: HashMap<String, HashMap<String, EntradaSentido>> =
            serde_json::from_reader(BufReader::new(f_lex))
                .map_err(|e| format!("parseando lexicon JSON: {}", e))?;

        let mut lexico: HashMap<String, HashMap<String, String>> = HashMap::new();
        for (palavra, sentidos) in lexico_completo {
            let mut m = HashMap::new();
            for (sentido, dados) in sentidos {
                m.insert(sentido, dados.ipa);
            }
            if !m.is_empty() {
                lexico.insert(palavra, m);
            }
        }

        Ok(Self {
            regras,
            lexico,
            caminho_regras: regras_path.display().to_string(),
            caminho_lexicon: lexicon_path.display().to_string(),
        })
    }

    pub fn n_regras(&self) -> usize {
        self.regras.len()
    }

    pub fn tem_regra(&self, palavra: &str) -> bool {
        self.regras.contains_key(palavra)
    }

    pub fn ipa_para(&self, palavra: &str, sentido: &str) -> Option<&str> {
        self.lexico
            .get(palavra)
            .and_then(|m| m.get(sentido))
            .map(|s| s.as_str())
    }

    pub fn desambiguar(
        &self,
        palavra: &str,
        sentenca: &str,
    ) -> Option<Disambiguacao> {
        let regra = self.regras.get(palavra)?;

        match regra {
            Regra::Single { sense, pos } => Some(Disambiguacao {
                sentido: sense.clone(),
                pos: pos.clone(),
                metodo: "single",
            }),
            Regra::Multi {
                senses,
                prior,
                feat_counts,
                feat_totals,
            } => {
                // 1. Expressões fixas contextuais.
                let toks = tokenize(sentenca);
                let tw = norm_word(palavra);
                if let Some(i) = toks.iter().position(|t| *t == tw) {
                    let prev = if i > 0 { toks[i - 1].as_str() } else { "" };
                    let next = if i + 1 < toks.len() { toks[i + 1].as_str() } else { "" };
                    let next2 = if i + 2 < toks.len() { toks[i + 2].as_str() } else { "" };
                    if let Some((sense, pos)) =
                        check_expressao_fixa(&tw, prev, next, next2)
                    {
                        return Some(Disambiguacao {
                            sentido: sense.to_string(),
                            pos: pos.to_string(),
                            metodo: "expressao_fixa",
                        });
                    }
                }

                // 2. Naive Bayes.
                let feats = match extract_features(sentenca, palavra) {
                    Some(f) => f,
                    None => {
                        let (best, _) = prior.iter().max_by(|a, b| {
                            a.1.partial_cmp(b.1)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })?;
                        let mut partes = best.splitn(2, '|');
                        let s = partes.next()?.to_string();
                        let p = partes.next().unwrap_or("").to_string();
                        return Some(Disambiguacao {
                            sentido: s,
                            pos: p,
                            metodo: "prior_only",
                        });
                    }
                };

                let mut scores: HashMap<&String, f64> = HashMap::new();
                for sense_key in senses {
                    let p_prior = prior.get(sense_key).copied().unwrap_or(1e-9);
                    let mut log_s = (p_prior + 1e-9).ln();
                    let fcounts = feat_counts.get(sense_key);
                    let ftot = feat_totals.get(sense_key).copied().unwrap_or(0) as f64;

                    for (fname, fval) in &feats {
                        if let Some(fc) = fcounts {
                            if let Some(by_val) = fc.get(*fname) {
                                let cnt = by_val.get(fval).copied().unwrap_or(0) as f64;
                                let prob = (cnt + 1.0) / (ftot + 5.0);
                                log_s += prob.ln();
                            }
                        }
                    }
                    scores.insert(sense_key, log_s);
                }

                let (best, _) = scores.iter().max_by(|a, b| {
                    a.1.partial_cmp(b.1)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })?;

                let mut partes = best.splitn(2, '|');
                let sentido = partes.next()?.to_string();
                let pos = partes.next().unwrap_or("").to_string();
                Some(Disambiguacao {
                    sentido,
                    pos,
                    metodo: "nb",
                })
            }
        }
    }

    pub fn diagnostico(&self) -> DiagnosticoHomografos {
        let mut single = 0;
        let mut multi = 0;
        for regra in self.regras.values() {
            match regra {
                Regra::Single { .. } => single += 1,
                Regra::Multi { .. } => multi += 1,
            }
        }

        let n_palavras_lexicon = self.lexico.len();
        let n_total_entradas_lexicon =
            self.lexico.values().map(|m| m.len()).sum::<usize>();

        let n_expressoes_fixas = EXPRESSOES_FIXAS.len();
        let n_expressoes_com_next2 = EXPRESSOES_FIXAS
            .iter()
            .filter(|(_, _, _, pr2, _, _)| pr2.is_some())
            .count();

        DiagnosticoHomografos {
            n_regras: self.regras.len(),
            regras_single: single,
            regras_multi: multi,
            n_palavras_lexicon,
            n_total_entradas_lexicon,
            n_expressoes_fixas,
            n_expressoes_com_next2,
            caminho_regras: self.caminho_regras.clone(),
            caminho_lexicon: self.caminho_lexicon.clone(),
        }
    }
}

/* ------------------------------------------------------------------ *
 * Testes
 * ------------------------------------------------------------------ */

#[cfg(test)]
mod testes {
    use super::*;

    fn regra_multi_simples() -> HashMap<String, Regra> {
        let mut regras = HashMap::new();
        regras.insert(
            "sede".to_string(),
            Regra::Multi {
                senses: vec!["seat|NOUN".to_string(), "thirst|NOUN".to_string()],
                prior: {
                    let mut m = HashMap::new();
                    m.insert("seat|NOUN".to_string(), 0.5);
                    m.insert("thirst|NOUN".to_string(), 0.5);
                    m
                },
                feat_counts: HashMap::new(),
                feat_totals: HashMap::new(),
            },
        );
        regras.insert(
            "único".to_string(),
            Regra::Single {
                sense: "only".to_string(),
                pos: "NOUN".to_string(),
            },
        );
        regras
    }

    fn lexicon_minimo() -> HashMap<String, HashMap<String, String>> {
        let mut lexico = HashMap::new();
        let mut sede = HashMap::new();
        sede.insert("seat".to_string(), "sˈɛdʒi".to_string());
        sede.insert("thirst".to_string(), "sˈedʒi".to_string());
        lexico.insert("sede".to_string(), sede);
        lexico
    }

    #[test]
    fn homografos_novo_funciona() {
        let h = Homografos::novo(regra_multi_simples(), lexicon_minimo());
        assert_eq!(h.n_regras(), 2);
        assert!(h.tem_regra("sede"));
        assert!(h.tem_regra("único"));
        assert!(!h.tem_regra("casa"));
    }

    #[test]
    fn ipa_para_funciona() {
        let h = Homografos::novo(regra_multi_simples(), lexicon_minimo());
        assert_eq!(h.ipa_para("sede", "seat"), Some("sˈɛdʒi"));
        assert_eq!(h.ipa_para("sede", "thirst"), Some("sˈedʒi"));
        assert_eq!(h.ipa_para("sede", "outro"), None);
        assert_eq!(h.ipa_para("casa", "seat"), None);
    }

    #[test]
    fn regra_single_devolve_sentido_fixo() {
        let h = Homografos::novo(regra_multi_simples(), lexicon_minimo());
        let r = h.desambiguar("único", "qualquer frase").unwrap();
        assert_eq!(r.sentido, "only");
        assert_eq!(r.metodo, "single");
    }

    #[test]
    fn expressao_fixa_pelo_menos() {
        assert_eq!(
            check_expressao_fixa("pelo", "", "menos", ""),
            Some(("by_the", "ADP"))
        );
        assert_eq!(
            check_expressao_fixa("pelo", "", "visto", ""),
            Some(("by_the", "ADP"))
        );
        assert_eq!(check_expressao_fixa("pelo", "", "outra", ""), None);
    }

    #[test]
    fn expressao_fixa_com_next2_molho_palha() {
        assert_eq!(
            check_expressao_fixa("molho", "", "de", "palha"),
            Some(("bundle", "NOUN"))
        );
        assert_eq!(
            check_expressao_fixa("molho", "", "de", "cogumelos"),
            Some(("sauce", "NOUN"))
        );
        assert_eq!(check_expressao_fixa("molho", "", "de", "outra"), None);
        assert_eq!(check_expressao_fixa("molho", "", "de", ""), None);
    }

    #[test]
    fn expressao_fixa_porto_sempre() {
        assert_eq!(
            check_expressao_fixa("porto", "sempre", "", ""),
            Some(("carry", "VERB"))
        );
        assert_eq!(check_expressao_fixa("porto", "outra", "", ""), None);
    }

    #[test]
    fn expressao_fixa_torno_ao() {
        assert_eq!(
            check_expressao_fixa("torno", "", "ao", ""),
            Some(("turn", "VERB"))
        );
        assert_eq!(
            check_expressao_fixa("torno", "", "à", ""),
            Some(("turn", "VERB"))
        );
    }

    #[test]
    fn expressao_fixa_colher_depoimentos() {
        assert_eq!(
            check_expressao_fixa("colher", "", "depoimentos", ""),
            Some(("harvest", "VERB"))
        );
        assert_eq!(
            check_expressao_fixa("colher", "", "notas", ""),
            Some(("harvest", "VERB"))
        );
    }

    #[test]
    fn extract_features_7_campos() {
        let feats = extract_features("Eu sopro vontade nos meus colegas", "sopro")
            .expect("features");
        assert_eq!(feats.len(), 7);
        let nomes: Vec<&str> = feats.iter().map(|(n, _)| *n).collect();
        assert!(nomes.contains(&"prev_word"));
        assert!(nomes.contains(&"next_word"));
        assert!(nomes.contains(&"next_word_2"));
    }

    #[test]
    fn extract_features_next2_correto() {
        let feats = extract_features("O molho de palha seca", "molho")
            .expect("features");
        let next = feats.iter().find(|(n, _)| *n == "next_word").map(|(_, v)| v.as_str());
        let next2 = feats.iter().find(|(n, _)| *n == "next_word_2").map(|(_, v)| v.as_str());
        assert_eq!(next, Some("de"));
        assert_eq!(next2, Some("palha"));
    }

    #[test]
    fn classify_token_categorias() {
        assert_eq!(classify_token("o"), "ART");
        assert_eq!(classify_token("de"), "PREP");
        assert_eq!(classify_token("eu"), "PRON");
        assert_eq!(classify_token("mas"), "CONJ");
        assert_eq!(classify_token("sempre"), "ADV");
        assert_eq!(classify_token("fazer"), "VERB_INF");
        assert_eq!(classify_token(""), "NONE");
    }

    #[test]
    fn diagnostico_conta_correto() {
        let h = Homografos::novo(regra_multi_simples(), lexicon_minimo());
        let d = h.diagnostico();
        assert_eq!(d.n_regras, 2);
        assert_eq!(d.regras_single, 1);
        assert_eq!(d.regras_multi, 1);
        assert!(d.n_expressoes_fixas > 0);
        assert!(d.n_expressoes_com_next2 > 0);
    }
}
