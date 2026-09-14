//! Desambiguador de homógrafos heterofônicos pt-BR.
//!
//! Usa Naive Bayes com features sintáticas (implementação do Bifonia
//! adaptada para pt-BR). As regras são carregadas de
//! `homograph_rules_v2.json` e o léxico `(palavra, sentido) → IPA`
//! de `lexicon_homografos.json`.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;

static RE_WORD: Lazy<Regex> = Lazy::new(|| Regex::new(r"\w+").unwrap());

#[derive(Deserialize, Debug)]
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

#[derive(Debug, Clone)]
pub struct Disambiguacao {
    pub sentido: String,
    #[allow(dead_code)]
    pub pos: String,
}

/// Desambiguador carregado de JSON.
pub struct Homografos {
    pub regras: HashMap<String, Regra>,
    /// `(palavra, sentido) → IPA`.
    pub lexico: HashMap<String, HashMap<String, String>>,
}

impl Homografos {
    pub fn novo(
        regras: HashMap<String, Regra>,
        lexico: HashMap<String, HashMap<String, String>>,
    ) -> Self {
        Self { regras, lexico }
    }

    pub fn tem_regra(&self, palavra: &str) -> bool {
        self.regras.contains_key(palavra)
    }

    /// Devolve o IPA associado ao par `(palavra, sentido)`.
    pub fn ipa_para(&self, palavra: &str, sentido: &str) -> Option<&str> {
        self.lexico
            .get(palavra)
            .and_then(|m| m.get(sentido))
            .map(|s| s.as_str())
    }

    /// Desambigua `palavra` no contexto de `sentenca`.
    pub fn desambiguar(&self, palavra: &str, sentenca: &str) -> Option<Disambiguacao> {
        let regra = self.regras.get(palavra)?;
        match regra {
            Regra::Single { sense, pos } => Some(Disambiguacao {
                sentido: sense.clone(),
                pos: pos.clone(),
            }),
            Regra::Multi {
                senses,
                prior,
                feat_counts,
                feat_totals,
            } => {
                let feats = extrair_features(sentenca, palavra)?;

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
                    a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                })?;
                let mut partes = best.splitn(2, '|');
                let sentido = partes.next()?.to_string();
                let pos = partes.next().unwrap_or("").to_string();
                Some(Disambiguacao { sentido, pos })
            }
        }
    }
}

fn extrair_features(sentenca: &str, alvo: &str) -> Option<Vec<(&'static str, String)>> {
    let toks: Vec<String> = RE_WORD
        .find_iter(sentenca)
        .map(|m| m.as_str().to_lowercase())
        .collect();
    let i = toks.iter().position(|t| t == alvo)?;

    let prev = if i > 0 { Some(toks[i - 1].as_str()) } else { None };
    let next = if i + 1 < toks.len() {
        Some(toks[i + 1].as_str())
    } else {
        None
    };

    let mut feats = Vec::with_capacity(6);
    feats.push(("prev_word", prev.unwrap_or("").to_string()));
    feats.push(("next_word", next.unwrap_or("").to_string()));
    feats.push(("prev_class", classificar(prev.unwrap_or("")).to_string()));
    feats.push(("next_class", classificar(next.unwrap_or("")).to_string()));
    feats.push((
        "is_first",
        if i == 0 { "true" } else { "false" }.to_string(),
    ));
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

const ARTIGOS: &[&str] = &["o", "a", "os", "as", "um", "uma", "uns", "umas"];
const DEMONSTR: &[&str] = &[
    "este", "esta", "estes", "estas", "esse", "essa", "esses", "essas",
    "isso", "isto", "aquele", "aquela", "aqueles", "aquelas", "aquilo",
];
const PREPOSICOES: &[&str] = &[
    "de", "do", "da", "dos", "das", "em", "no", "na", "nos", "nas",
    "por", "pelo", "pela", "pelos", "pelas", "com", "sem", "para",
    "ao", "à", "aos", "às", "entre", "sobre", "sob", "contra", "desde", "até",
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

fn classificar(t: &str) -> &'static str {
    if t.is_empty() {
        return "NONE";
    }
    if ARTIGOS.contains(&t) {
        return "ART";
    }
    if DEMONSTR.contains(&t) {
        return "DEM";
    }
    if PREPOSICOES.contains(&t) {
        return "PREP";
    }
    if PRONOMES.contains(&t) {
        return "PRON";
    }
    if CONJUNCOES.contains(&t) {
        return "CONJ";
    }
    if ADVERBIOS.contains(&t) {
        return "ADV";
    }
    if t.len() > 3 && (t.ends_with("ar") || t.ends_with("er") || t.ends_with("ir")) {
        return "VERB_INF";
    }
    if t.len() > 4 && (t.ends_with("ando") || t.ends_with("endo") || t.ends_with("indo")) {
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