//! Desambiguador de homógrafos heterofônicos pt-BR.
//!
//! Usa três níveis de decisão, em ordem de prioridade:
//!
//! 1. **Expressões fixas** (1-gram): padrões do tipo "`pelo visto`",
//!    "`acerto` depois de `nunca`", etc. Definidos em `EXPRESSOES_FIXAS`.
//!
//! 2. **Bigramas** (2-gram): quando a palavra decisiva está a 2 posições
//!    de distância. Definidos em `src/bigrama.rs`.
//!
//! 3. **Naive Bayes** com features sintáticas (adaptação do Bifonia):
//!    `prev_word`, `next_word`, `prev_class`, `next_class`, `is_first`,
//!    `pos_in_sent`.
//!
//! O léxico `(palavra, sentido) → IPA` é carregado de
//! `lexicon_homografos.json`.

use crate::bigrama;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// Regex para tokenização (compilada uma vez por chamada, não
/// armazenada como estático para evitar dependência de once_cell).
fn regex_palavra() -> Regex {
    Regex::new(r"\w+").unwrap()
}

// ---------------------------------------------------------------------------
// Tipos públicos
// ---------------------------------------------------------------------------

/// Resultado de uma desambiguação.
#[derive(Debug, Clone)]
pub struct Disambiguacao {
    /// Sentido ativo na sentença (ex: `"thirst"`, `"seat"`).
    pub sentido: String,
    /// Classe gramatical (ex: `"NOUN"`, `"VERB"`, `"ADJ"`).
    pub pos: String,
}

/// Regra treinada: `single` (uma leitura só) ou `multi` (múltiplas
/// leituras com prior e contagens de features).
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

/// Entrada do léxico `(palavra, sentido) → IPA`.
#[derive(Deserialize)]
struct EntradaSentido {
    ipa: String,
    #[serde(default)]
    #[allow(dead_code)]
    confianca: f64,
    #[serde(default)]
    #[allow(dead_code)]
    ocorrencias: usize,
}

/// Desambiguador carregado de JSON.
pub struct Homografos {
    pub regras: HashMap<String, Regra>,
    pub lexico: HashMap<String, HashMap<String, String>>,
}

impl Homografos {
    /// Constrói a partir de mapas já parseados.
    pub fn novo(
        regras: HashMap<String, Regra>,
        lexico: HashMap<String, HashMap<String, String>>,
    ) -> Self {
        Self { regras, lexico }
    }

    /// Carrega de dois arquivos JSON.
    ///
    /// - `regras_path` — `homograph_rules_v2.json`
    /// - `lexico_path` — `lexicon_homografos.json`
    ///
    /// O léxico deve ter o formato `{ palavra: { sentido: { ipa, ... } } }`.
    pub fn from_paths(
        regras_path: &Path,
        lexico_path: &Path,
    ) -> Result<Self, String> {
        let regras_texto = std::fs::read_to_string(regras_path).map_err(|erro| {
            format!("Erro ao ler {}: {}", regras_path.display(), erro)
        })?;
        let regras: HashMap<String, Regra> = serde_json::from_str(&regras_texto)
            .map_err(|erro| format!("Erro ao parsear regras: {}", erro))?;

        let lexico_texto = std::fs::read_to_string(lexico_path).map_err(|erro| {
            format!("Erro ao ler {}: {}", lexico_path.display(), erro)
        })?;
        let lexico_completo: HashMap<String, HashMap<String, EntradaSentido>> =
            serde_json::from_str(&lexico_texto)
                .map_err(|erro| format!("Erro ao parsear léxico: {}", erro))?;

        let mut lexico: HashMap<String, HashMap<String, String>> = HashMap::new();
        for (palavra, sentidos) in lexico_completo {
            let mut mapa_sentidos = HashMap::new();
            for (sentido, dados) in sentidos {
                mapa_sentidos.insert(sentido, dados.ipa);
            }
            if !mapa_sentidos.is_empty() {
                lexico.insert(palavra, mapa_sentidos);
            }
        }

        Ok(Self::novo(regras, lexico))
    }

    /// Diz se a palavra tem regra NB **ou** bigrama cadastrado.
    pub fn tem_regra(&self, palavra: &str) -> bool {
        self.regras.contains_key(palavra) || bigrama::tem_bigrama(palavra)
    }

    /// Devolve o IPA associado ao par `(palavra, sentido)`.
    pub fn ipa_para(&self, palavra: &str, sentido: &str) -> Option<&str> {
        self.lexico
            .get(palavra)
            .and_then(|mapa| mapa.get(sentido))
            .map(|texto| texto.as_str())
    }

    /// Desambigua `palavra` no contexto de `sentenca`.
    ///
    /// Ordem de decisão:
    /// 1. Expressão fixa (1-gram)
    /// 2. Bigrama forward (2-gram: palavra + next1 + next2)
    /// 3. Bigrama backward (2-gram: prev2 + prev1 + palavra)
    /// 4. Naive Bayes
    pub fn desambiguar(
        &self,
        palavra: &str,
        sentenca: &str,
    ) -> Option<Disambiguacao> {
        if let Some(d) = self.desambiguar_expressao_fixa(palavra, sentenca) {
            return Some(d);
        }
        if let Some(d) = self.desambiguar_bigrama_next(palavra, sentenca) {
            return Some(d);
        }
        if let Some(d) = self.desambiguar_bigrama_prev(palavra, sentenca) {
            return Some(d);
        }
        self.desambiguar_nb(palavra, sentenca)
    }

    // -----------------------------------------------------------------------
    // Nível 1 — Expressões fixas (1-gram)
    // -----------------------------------------------------------------------

    fn desambiguar_expressao_fixa(
        &self,
        palavra: &str,
        sentenca: &str,
    ) -> Option<Disambiguacao> {
        let re = regex_palavra();
        let tokens: Vec<String> = re
            .find_iter(sentenca)
            .map(|m| m.as_str().to_lowercase())
            .collect();
        let indice = tokens.iter().position(|t| t == palavra)?;
        let anterior = if indice > 0 {
            tokens[indice - 1].as_str()
        } else {
            ""
        };
        let proxima = if indice + 1 < tokens.len() {
            tokens[indice + 1].as_str()
        } else {
            ""
        };
        let (sentido, pos) = verificar_expressao_fixa(palavra, anterior, proxima)?;
        Some(Disambiguacao {
            sentido: sentido.to_string(),
            pos: pos.to_string(),
        })
    }

    // -----------------------------------------------------------------------
    // Nível 2 — Bigrama forward
    // -----------------------------------------------------------------------

    fn desambiguar_bigrama_next(
        &self,
        palavra: &str,
        sentenca: &str,
    ) -> Option<Disambiguacao> {
        let re = regex_palavra();
        let tokens: Vec<String> = re
            .find_iter(sentenca)
            .map(|m| m.as_str().to_lowercase())
            .collect();
        let indice = tokens.iter().position(|t| t == palavra)?;
        if indice + 2 >= tokens.len() {
            return None;
        }
        let proxima_1 = tokens[indice + 1].as_str();
        let proxima_2 = tokens[indice + 2].as_str();
        let sentido = bigrama::verificar_next(palavra, proxima_1, proxima_2)?;
        Some(Disambiguacao {
            sentido: sentido.to_string(),
            pos: String::new(),
        })
    }

    // -----------------------------------------------------------------------
    // Nível 3 — Bigrama backward
    // -----------------------------------------------------------------------

    fn desambiguar_bigrama_prev(
        &self,
        palavra: &str,
        sentenca: &str,
    ) -> Option<Disambiguacao> {
        let re = regex_palavra();
        let tokens: Vec<String> = re
            .find_iter(sentenca)
            .map(|m| m.as_str().to_lowercase())
            .collect();
        let indice = tokens.iter().position(|t| t == palavra)?;
        if indice < 2 {
            return None;
        }
        let anterior_2 = tokens[indice - 2].as_str();
        let anterior_1 = tokens[indice - 1].as_str();
        let sentido = bigrama::verificar_prev(palavra, anterior_2, anterior_1)?;
        Some(Disambiguacao {
            sentido: sentido.to_string(),
            pos: String::new(),
        })
    }

    // -----------------------------------------------------------------------
    // Nível 4 — Naive Bayes
    // -----------------------------------------------------------------------

    fn desambiguar_nb(
        &self,
        palavra: &str,
        sentenca: &str,
    ) -> Option<Disambiguacao> {
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
                let features = extrair_features(sentenca, palavra);

                let features = match features {
                    Some(f) => f,
                    None => {
                        // Sem contexto: devolve o sentido mais frequente.
                        let melhor = prior.iter().max_by(|a, b| {
                            a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                        })?;
                        let mut partes = melhor.0.splitn(2, '|');
                        let sentido = partes.next()?.to_string();
                        let pos = partes.next().unwrap_or("").to_string();
                        return Some(Disambiguacao { sentido, pos });
                    }
                };

                let mut pontuacoes: HashMap<&String, f64> = HashMap::new();
                for chave_sentido in senses {
                    let p_prior = prior.get(chave_sentido).copied().unwrap_or(1e-9);
                    let mut log_pontuacao = (p_prior + 1e-9).ln();
                    let contagens_feature = feat_counts.get(chave_sentido);
                    let total_feature =
                        feat_totals.get(chave_sentido).copied().unwrap_or(0) as f64;

                    for (nome_feature, valor_feature) in &features {
                        if let Some(contagens) = contagens_feature {
                            if let Some(por_valor) = contagens.get(*nome_feature) {
                                let contagem = por_valor
                                    .get(valor_feature)
                                    .copied()
                                    .unwrap_or(0) as f64;
                                let probabilidade =
                                    (contagem + 1.0) / (total_feature + 5.0);
                                log_pontuacao += probabilidade.ln();
                            }
                        }
                    }
                    pontuacoes.insert(chave_sentido, log_pontuacao);
                }

                let melhor = pontuacoes.iter().max_by(|a, b| {
                    a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
                })?;
                let mut partes = melhor.0.splitn(2, '|');
                let sentido = partes.next()?.to_string();
                let pos = partes.next().unwrap_or("").to_string();
                Some(Disambiguacao { sentido, pos })
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Features sintáticas
// ---------------------------------------------------------------------------

fn extrair_features(
    sentenca: &str,
    alvo: &str,
) -> Option<Vec<(&'static str, String)>> {
    let re = regex_palavra();
    let tokens: Vec<String> = re
        .find_iter(sentenca)
        .map(|m| m.as_str().to_lowercase())
        .collect();
    let indice = tokens.iter().position(|t| t == alvo)?;

    let anterior = if indice > 0 {
        Some(tokens[indice - 1].as_str())
    } else {
        None
    };
    let proxima = if indice + 1 < tokens.len() {
        Some(tokens[indice + 1].as_str())
    } else {
        None
    };

    let mut features: Vec<(&'static str, String)> = Vec::with_capacity(6);
    features.push(("prev_word", anterior.unwrap_or("").to_string()));
    features.push(("next_word", proxima.unwrap_or("").to_string()));
    features.push((
        "prev_class",
        classificar_token(anterior.unwrap_or("")).to_string(),
    ));
    features.push((
        "next_class",
        classificar_token(proxima.unwrap_or("")).to_string(),
    ));
    features.push((
        "is_first",
        if indice == 0 { "true" } else { "false" }.to_string(),
    ));
    let posicao_na_sentenca = if indice == 0 {
        "first"
    } else if indice == tokens.len() - 1 {
        "last"
    } else {
        "middle"
    };
    features.push(("pos_in_sent", posicao_na_sentenca.to_string()));
    Some(features)
}

const ARTIGOS: &[&str] = &["o", "a", "os", "as", "um", "uma", "uns", "umas"];
const DEMONSTRATIVOS: &[&str] = &[
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

fn classificar_token(token: &str) -> &'static str {
    if token.is_empty() {
        return "NONE";
    }
    if ARTIGOS.contains(&token) {
        return "ART";
    }
    if DEMONSTRATIVOS.contains(&token) {
        return "DEM";
    }
    if PREPOSICOES.contains(&token) {
        return "PREP";
    }
    if PRONOMES.contains(&token) {
        return "PRON";
    }
    if CONJUNCOES.contains(&token) {
        return "CONJ";
    }
    if ADVERBIOS.contains(&token) {
        return "ADV";
    }
    if token.len() > 3
        && (token.ends_with("ar") || token.ends_with("er") || token.ends_with("ir"))
    {
        return "VERB_INF";
    }
    if token.len() > 4
        && (token.ends_with("ando") || token.ends_with("endo") || token.ends_with("indo"))
    {
        return "VERB_GER";
    }
    if token.len() > 4 && (token.ends_with("ado") || token.ends_with("ido")) {
        return "VERB_PART";
    }
    if token.len() > 6 && token.ends_with("mente") {
        return "ADV";
    }
    "OTHER"
}

// ---------------------------------------------------------------------------
// Expressões fixas (1-gram)
// ---------------------------------------------------------------------------

/// Formato: `(palavra, anterior_opcional, próxima_opcional, sentido, pos)`.
///
/// AMBOS os lados `None` nunca acontecem — sempre exige contexto.
static EXPRESSOES_FIXAS: &[(&str, Option<&str>, Option<&str>, &str, &str)] = &[
    // ─── pelo + X (existing) ───
    ("pelo", None, Some("visto"), "by_the", "ADP"),
    ("pelo", None, Some("menos"), "by_the", "ADP"),
    ("pelo", None, Some("contrário"), "by_the", "ADP"),
    ("pelo", None, Some("contrario"), "by_the", "ADP"),
    ("pelo", None, Some("amor"), "by_the", "ADP"),
    ("pelo", None, Some("jeito"), "by_the", "ADP"),
    ("pelo", None, Some("mundo"), "by_the", "ADP"),
    ("pelo", None, Some("fato"), "by_the", "ADP"),
    ("pelo", None, Some("caminho"), "by_the", "ADP"),
    ("pelo", None, Some("tempo"), "by_the", "ADP"),
    ("pelo", None, Some("silêncio"), "by_the", "ADP"),
    ("pelo", None, Some("silencio"), "by_the", "ADP"),
    ("pelo", None, Some("que"), "by_the", "ADP"),

    // ─── pela + X (existing) ───
    ("pela", None, Some("manhã"), "by_the", "ADP"),
    ("pela", None, Some("manha"), "by_the", "ADP"),
    ("pela", None, Some("tarde"), "by_the", "ADP"),
    ("pela", None, Some("noite"), "by_the", "ADP"),
    ("pela", None, Some("primeira"), "by_the", "ADP"),
    ("pela", None, Some("última"), "by_the", "ADP"),
    ("pela", None, Some("ultima"), "by_the", "ADP"),

    // ═══════════════════════════════════════════════════════════════════════
    // NOVAS — verbos 1sg com sujeito/advérbio explícito
    // ═══════════════════════════════════════════════════════════════════════

    // porto (verbo portar, 1sg)
    ("porto", Some("sempre"), None, "carry", "VERB"),
    ("porto", Some("eu"), None, "carry", "VERB"),
    ("porto", None, Some("atitudes"), "carry", "VERB"),
    ("porto", None, Some("verdades"), "carry", "VERB"),

    // sopro (verbo soprar, 1sg)
    ("sopro", Some("eu"), None, "blow", "VERB"),
    ("sopro", None, Some("vontade"), "blow", "VERB"),

    // gozo (verbo gozar, 1sg) — "gozo de X"
    ("gozo", None, Some("de"), "enjoy", "VERB"),

    // rego (verbo regar, 1sg)
    ("rego", Some("tempo"), None, "water", "VERB"),
    ("rego", Some("eu"), None, "water", "VERB"),

    // desapego (verbo desapegar, 1sg)
    ("desapego", Some("eu"), None, "let_go", "VERB"),

    // despojo (verbo despojar, 1sg)
    ("despojo", Some("nunca"), None, "strip", "VERB"),
    ("despojo", Some("eu"), None, "strip", "VERB"),

    // acerto (verbo acertar, 1sg)
    ("acerto", Some("nunca"), None, "adjust", "VERB"),
    ("acerto", Some("sempre"), None, "adjust", "VERB"),

    // solto (verbo soltar, 1sg)
    ("solto", Some("eu"), None, "release", "VERB"),

    // desaforo (verbo desaforar, 1sg)
    ("desaforo", Some("posso"), None, "affront", "VERB"),

    // congelo (verbo congelar, 1sg)
    ("congelo", Some("quando"), None, "freeze", "VERB"),

    // desconforto (verbo desconfortar, 1sg)
    ("desconforto", Some("não"), None, "discomfit", "VERB"),
    ("desconforto", Some("nao"), None, "discomfit", "VERB"),

    // desassossego (verbo desassossegar, 1sg)
    ("desassossego", Some("não"), None, "disturb", "VERB"),
    ("desassossego", Some("nao"), None, "disturb", "VERB"),

    // torno (verbo tornar, 1sg) — "torno ao/à X"
    ("torno", None, Some("ao"), "turn", "VERB"),
    ("torno", None, Some("à"), "turn", "VERB"),

    // ═══════════════════════════════════════════════════════════════════════
    // NOVAS — substantivos/adjetivos por contexto imediato
    // ═══════════════════════════════════════════════════════════════════════

    // cor (substantivo) — "cor exata", "cor ideal"
    ("cor", None, Some("exata"), "colour", "NOUN"),
    ("cor", None, Some("ideal"), "colour", "NOUN"),

    // sede (substantivo local) — "nova sede"
    ("sede", Some("nova"), None, "seat", "NOUN"),

    // lobo (animal) — "do lobo"
    ("lobo", Some("do"), None, "wolf", "NOUN"),

    // torre (substantivo) — "torre eólica"
    ("torre", None, Some("eólica"), "tower", "NOUN"),
    ("torre", None, Some("eolica"), "tower", "NOUN"),

    // colher (verbo) — "colher depoimentos/notas"
    ("colher", None, Some("depoimentos"), "harvest", "VERB"),
    ("colher", None, Some("notas"), "harvest", "VERB"),
    ("colher", None, Some("informações"), "harvest", "VERB"),
    ("colher", None, Some("dados"), "harvest", "VERB"),
];

fn verificar_expressao_fixa(
    palavra: &str,
    anterior: &str,
    proxima: &str,
) -> Option<(&'static str, &'static str)> {
    for (palavra_alvo, anterior_opcional, proxima_opcional, sentido, pos) in EXPRESSOES_FIXAS
    {
        if *palavra_alvo != palavra {
            continue;
        }
        let ok_anterior = anterior_opcional.map_or(true, |x| x == anterior);
        let ok_proxima = proxima_opcional.map_or(true, |x| x == proxima);
        if ok_anterior && ok_proxima {
            return Some((sentido, pos));
        }
    }
    None
}

#[cfg(test)]
mod testes {
    use super::*;

    fn homografos_de_teste() -> Homografos {
        // Regras de teste mínimas
        let mut regras: HashMap<String, Regra> = HashMap::new();
        regras.insert(
            "sede".to_string(),
            Regra::Multi {
                senses: vec!["seat|NOUN".to_string(), "thirst|NOUN".to_string()],
                prior: {
                    let mut p = HashMap::new();
                    p.insert("seat|NOUN".to_string(), 0.5);
                    p.insert("thirst|NOUN".to_string(), 0.5);
                    p
                },
                feat_counts: HashMap::new(),
                feat_totals: HashMap::new(),
            },
        );
        Homografos::novo(regras, HashMap::new())
    }

    #[test]
    fn tem_regra_detecta_palavra_com_regra_nb() {
        let h = homografos_de_teste();
        assert!(h.tem_regra("sede"));
        assert!(!h.tem_regra("casa"));
    }

    #[test]
    fn tem_regra_detecta_palavra_com_bigrama() {
        let h = homografos_de_teste();
        assert!(h.tem_regra("molho"));
        assert!(h.tem_regra("colher"));
        assert!(h.tem_regra("bola"));
    }

    #[test]
    fn desambigua_por_expressao_fixa() {
        let h = homografos_de_teste();
        let resultado = h.desambiguar("pelo", "pelo visto, ele veio");
        assert!(resultado.is_some());
        let d = resultado.unwrap();
        assert_eq!(d.sentido, "by_the");
    }

    #[test]
    fn desambigua_por_bigrama_next() {
        let h = homografos_de_teste();
        let resultado = h.desambiguar("molho", "o molho de palha seca");
        assert!(resultado.is_some());
        let d = resultado.unwrap();
        assert_eq!(d.sentido, "bundle");

        let resultado2 = h.desambiguar("molho", "um molho de cogumelos");
        assert!(resultado2.is_some());
        assert_eq!(resultado2.unwrap().sentido, "sauce");
    }

    #[test]
    fn desambigua_por_bigrama_prev() {
        let h = homografos_de_teste();
        let resultado = h.desambiguar("colher", "a equipa começou a colher depoimentos");
        assert!(resultado.is_some());
        assert_eq!(resultado.unwrap().sentido, "harvest");
    }

    #[test]
    fn desambigua_por_nb_quando_sem_regra_especifica() {
        let h = homografos_de_teste();
        let resultado = h.desambiguar("sede", "tenho sede de água");
        assert!(resultado.is_some());
        // Sem features no teste, cai no prior (ambos 0.5)
    }

    #[test]
    fn palavra_desconhecida_retorna_none() {
        let h = homografos_de_teste();
        assert!(h.desambiguar("inexistente", "frase qualquer").is_none());
    }
}
