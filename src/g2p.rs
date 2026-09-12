//! Conversor grafema→fonema pt-BR.
//!
//! Porte de `index.js` do Vozz, com correções alinhadas ao espeak-ng pt-br.
//!
//! Regras de nasalização:
//!
//! | Vogal | Antes de coda m/n | Antes de nasal no onset (m/n/nh) |
//! |-------|-------------------|----------------------------------|
//! | `a`/`â`| `ɐ̃`              | `ɐ̃` (tônico, ou pré-tônico antes de `nh`) / `æ` (pré-tônico antes de `m`/`n`) |
//! | `e`   | `eɪ`              | `e` puro                          |
//! | `i`   | `i` puro          | `i` puro                          |
//! | `o`   | `o` puro          | `o` puro                          |
//! | `u`   | `ũ`               | `ũ` (tônico, ou antes de `nh`)     |
//!
//! Ditongo crescente (`i`/`u` + vogal forte):
//!
//! - Sílaba tônica → hiato.
//! - Segunda vogal com acento gráfico → hiato forçado.
//! - Consoante palatalizável (`c`/`g`/`d`/`t` brando) antes do `i` → hiato.
//!
//! Acentuação:
//!
//! - `-is`/`-us` final de verbo → oxítona.
//! - `-om`/`-um` final (2+ sílabas) → oxítona.
//! - `-irdes` final → tônica no `i`.
//! - Prefixo `sobre-` (4+ sílabas) → sem acento secundário inicial.
//!
//! Notação:
//!
//! - `y` final após `tʃ`/`dʒ` → `j`.
//! - `ʊ` final após `ʃ`/`ʒ`/`z` → `w`.
//!
//! Clíticos contextuais (ver `lexicon.rs`):
//!
//! - Palavras funcionais em contexto perdem acento primário.
//!
//! Sândi:
//!
//! - `s` final antes de vogal ou consoante sonora → `z`.
//!
//! Ordem de resolução em `resolver_palavra`:
//!
//! 1. Clíticos contextuais (mais específico).
//! 2. Clíticos átonos gerais.
//! 3. Léxico interno (correto por construção).
//! 4. Léxico auto-carregado (JSON do espeak).
//! 5. Regras.

use crate::lexicon::{
    buscar_clitico, buscar_clitico_contexto, buscar_lexico,
};
use crate::normalize::{normalizar, OpcoesNormalizar};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

/* ------------------------------------------------------------------ *
 * Tabelas básicas
 * ------------------------------------------------------------------ */

#[allow(dead_code)]
const FORTES: &str = "aeoáéóâêôãõà";

const FRACAS: &str = "iu";

const VOGAIS: &str = "aeoáéóâêôãõàiuíúïü";
const ACENTO_GRAFICO: &str = "áéíóúâêô";
const TIL: &str = "ãõ";

const DIGRAFOS: [&str; 5] = ["ch", "lh", "nh", "rr", "ss"];
const OBSTRUINTES: &str = "pbtdkgfvc";
const NASALIZAVEIS: [&str; 3] = ["m", "n", "nh"];

const PALATALIZAVEIS: [&str; 4] = ["c", "g", "d", "t"];

const RADICAIS_KS: &[&str] = &[
    "taxi", "fix", "sex", "toxic", "reflex", "complex", "anex",
    "flux", "nex", "paradox", "ortodox", "prolix", "axiom", "axil",
    "asfixi", "crucifix", "toxin", "elix", "climax",
    "hidrox", "carbox", "oxid", "oxig", "oxil", "dioxid",
    "peroxid", "superoxid",
    "hexa", "flex",
];

const EXCECOES_KS: [&str; 2] = ["sext", "anexim"];

fn tem_radical_ks(palavra: &str) -> bool {
    let normalizada: String = palavra
        .to_lowercase()
        .nfd()
        .filter(|c| !('\u{0300}'..='\u{036F}').contains(c))
        .collect();
    for excecao in EXCECOES_KS {
        if normalizada.starts_with(excecao) {
            return false;
        }
    }
    RADICAIS_KS.iter().any(|radical| normalizada.contains(radical))
}

fn eh_vogal(c: char) -> bool {
    VOGAIS.contains(c)
}

#[allow(dead_code)]
fn eh_forte(c: char) -> bool {
    FORTES.contains(c)
}

fn eh_fraca(c: char) -> bool {
    FRACAS.contains(c)
}

fn eh_fraca_acentuada(c: char) -> bool {
    "íúï".contains(c)
}

fn tem_til_grafico(c: char) -> bool {
    TIL.contains(c)
}

fn tem_acento_grafico(c: char) -> bool {
    ACENTO_GRAFICO.contains(c)
}

fn palavra_tem_acento_grafico(palavra: &str) -> bool {
    palavra.chars().any(|c| ACENTO_GRAFICO.contains(c))
}

fn eh_oxitona_is_us(palavra: &str) -> bool {
    if palavra_tem_acento_grafico(palavra) {
        return false;
    }
    let p = palavra.to_lowercase();
    (p.ends_with("is") || p.ends_with("us")) && p.chars().count() >= 3
}

fn eh_oxitona_om_um(palavra: &str, n_silabas: usize) -> bool {
    if n_silabas < 2 || palavra_tem_acento_grafico(palavra) {
        return false;
    }
    let p = palavra.to_lowercase();
    p.ends_with("om") || p.ends_with("um")
}

fn eh_oxitona_irdes(palavra: &str) -> bool {
    let p = palavra.to_lowercase();
    p.ends_with("irdes") && p.chars().count() >= 5
}

fn tem_prefixo_sobre(palavra: &str, n_silabas: usize) -> bool {
    n_silabas >= 4 && palavra.to_lowercase().starts_with("sobre")
}

/* ------------------------------------------------------------------ *
 * 1. Segmentação em unidades
 * ------------------------------------------------------------------ */

#[derive(Clone, Copy, PartialEq, Eq)]
enum TipoUnidade {
    Consoante,
    Vogal,
}

#[derive(Clone)]
struct Unidade {
    tipo: TipoUnidade,
    texto: String,
}

fn segmentar(palavra: &str) -> Vec<Unidade> {
    let caracteres: Vec<char> = palavra.chars().collect();
    let mut unidades: Vec<Unidade> = Vec::new();
    let mut indice = 0;

    while indice < caracteres.len() {
        let caractere = caracteres[indice];

        if eh_vogal(caractere) {
            unidades.push(Unidade {
                tipo: TipoUnidade::Vogal,
                texto: caractere.to_string(),
            });
            indice += 1;
            continue;
        }

        let par = if indice + 1 < caracteres.len() {
            format!("{}{}", caracteres[indice], caracteres[indice + 1])
        } else {
            String::new()
        };
        let proximo_apos_par = caracteres.get(indice + 2).copied();

        if par == "gu" {
            match proximo_apos_par {
                Some('a') | Some('o') | Some('i') => {
                    unidades.push(Unidade {
                        tipo: TipoUnidade::Consoante,
                        texto: "gw".to_string(),
                    });
                    indice += 2;
                    continue;
                }
                Some('e') | Some('é') | Some('ê') => {
                    unidades.push(Unidade {
                        tipo: TipoUnidade::Consoante,
                        texto: "gu".to_string(),
                    });
                    indice += 2;
                    continue;
                }
                _ => {}
            }
        }

        if par == "qu" {
            match proximo_apos_par {
                Some('a') | Some('o') => {
                    unidades.push(Unidade {
                        tipo: TipoUnidade::Consoante,
                        texto: "kw".to_string(),
                    });
                    indice += 2;
                    continue;
                }
                Some('e') | Some('é') | Some('ê') | Some('i') | Some('í') => {
                    unidades.push(Unidade {
                        tipo: TipoUnidade::Consoante,
                        texto: "qu".to_string(),
                    });
                    indice += 2;
                    continue;
                }
                _ => {}
            }
        }

        if DIGRAFOS.contains(&par.as_str()) {
            unidades.push(Unidade {
                tipo: TipoUnidade::Consoante,
                texto: par,
            });
            indice += 2;
            continue;
        }

        unidades.push(Unidade {
            tipo: TipoUnidade::Consoante,
            texto: caractere.to_string(),
        });
        indice += 1;
    }

    unidades
}

/* ------------------------------------------------------------------ *
 * 2. Silabificação
 * ------------------------------------------------------------------ */

#[derive(Clone, Default)]
pub struct Silaba {
    pub onset: Vec<String>,
    pub nucleo: Vec<String>,
    pub coda: Vec<String>,
    pub tonica: bool,
    pub secundaria: bool,
}

pub fn silabificar(palavra: &str) -> Vec<Silaba> {
    let palavra_minuscula = palavra.to_lowercase();
    let unidades = segmentar(&palavra_minuscula);
    if unidades.is_empty() {
        return Vec::new();
    }

    let tem_acento = palavra_tem_acento_grafico(&palavra_minuscula);

    let mut nucleos: Vec<Vec<String>> = Vec::new();
    let mut consoantes_antes: Vec<Vec<String>> = Vec::new();
    let mut buffer: Vec<String> = Vec::new();

    let mut indice = 0;
    while indice < unidades.len() {
        let unidade = &unidades[indice];

        if unidade.tipo == TipoUnidade::Consoante {
            buffer.push(unidade.texto.clone());
            indice += 1;
            continue;
        }

        let mut vogais = vec![unidade.texto.clone()];
        let atual_c = unidade.texto.chars().next().unwrap_or(' ');

        let consoante_antes = buffer.last().cloned().unwrap_or_default();

        if let Some(proxima) = unidades.get(indice + 1) {
            if proxima.tipo == TipoUnidade::Vogal {
                let prox_c = proxima.texto.chars().next().unwrap_or(' ');

                let ditongo_nasal_grafico = tem_til_grafico(atual_c);

                let eh_fraca_prox = eh_fraca(prox_c);
                let ambas_fracas = eh_fraca(atual_c) && eh_fraca(prox_c);
                let prox_prox_e_vogal = unidades
                    .get(indice + 2)
                    .map_or(false, |d| d.tipo == TipoUnidade::Vogal);
                let ditongo_decrescente =
                    eh_fraca_prox && !(ambas_fracas && prox_prox_e_vogal);

                let prox_e_forte =
                    !eh_fraca(prox_c) && !eh_fraca_acentuada(prox_c);
                let prox_tem_acento = tem_acento_grafico(prox_c);
                let i_e_palatalizado = (atual_c == 'i' || atual_c == 'í')
                    && PALATALIZAVEIS.contains(&consoante_antes.as_str());
                let ditongo_crescente = eh_fraca(atual_c)
                    && prox_e_forte
                    && tem_acento
                    && !i_e_palatalizado
                    && !prox_tem_acento;

                if ditongo_nasal_grafico || ditongo_decrescente || ditongo_crescente {
                    vogais.push(proxima.texto.clone());
                    indice += 1;
                }
            }
        }

        nucleos.push(vogais);
        consoantes_antes.push(buffer.clone());
        buffer.clear();
        indice += 1;
    }
    let consoantes_finais = buffer;

    if eh_oxitona_irdes(&palavra_minuscula) && nucleos.len() >= 2 {
        let idx = nucleos.len() - 2;
        let nuc = nucleos[idx].clone();
        if nuc.len() == 2 && (nuc[1] == "i" || nuc[1] == "í") {
            let v1 = nuc[0].clone();
            let v2 = nuc[1].clone();
            nucleos[idx] = vec![v1];
            nucleos.insert(idx + 1, vec![v2]);
            consoantes_antes.insert(idx + 1, Vec::new());
        }
    }

    if nucleos.is_empty() {
        return vec![Silaba {
            onset: Vec::new(),
            nucleo: Vec::new(),
            coda: consoantes_finais,
            tonica: true,
            secundaria: false,
        }];
    }

    let mut silabas: Vec<Silaba> = Vec::with_capacity(nucleos.len());
    for nucleo in &nucleos {
        silabas.push(Silaba {
            onset: Vec::new(),
            nucleo: nucleo.clone(),
            coda: Vec::new(),
            tonica: false,
            secundaria: false,
        });
    }
    silabas[0].onset = consoantes_antes[0].clone();

    for posicao in 1..nucleos.len() {
        let grupo = &consoantes_antes[posicao];
        if grupo.is_empty() {
            continue;
        }
        if grupo.len() == 1 {
            silabas[posicao].onset = vec![grupo[0].clone()];
            continue;
        }

        let ultima = &grupo[grupo.len() - 1];
        let penultima = &grupo[grupo.len() - 2];
        let cluster_valido = (ultima == "l" || ultima == "r")
            && penultima.chars().count() == 1
            && penultima
                .chars()
                .next()
                .map_or(false, |c| OBSTRUINTES.contains(c));

        if cluster_valido {
            silabas[posicao - 1].coda = grupo[..grupo.len() - 2].to_vec();
            silabas[posicao].onset = vec![penultima.clone(), ultima.clone()];
        } else {
            silabas[posicao - 1].coda = grupo[..grupo.len() - 1].to_vec();
            silabas[posicao].onset = vec![ultima.clone()];
        }
    }
    let ultima_posicao = silabas.len() - 1;
    silabas[ultima_posicao].coda = consoantes_finais;

    silabas
}

/* ------------------------------------------------------------------ *
 * 3. Tonicidade
 * ------------------------------------------------------------------ */

pub fn acentuar(silabas: &mut [Silaba], palavra: &str) -> i32 {
    if silabas.is_empty() {
        return -1;
    }

    for indice in 0..silabas.len() {
        if silabas[indice]
            .nucleo
            .iter()
            .any(|v| v.chars().next().map_or(false, |c| ACENTO_GRAFICO.contains(c)))
        {
            silabas[indice].tonica = true;
            return indice as i32;
        }
    }
    if silabas.len() == 1 {
        silabas[0].tonica = true;
        return 0;
    }

    let ultima = silabas.len() - 1;
    if silabas[ultima]
        .nucleo
        .iter()
        .any(|v| v.chars().next().map_or(false, |c| TIL.contains(c)))
    {
        silabas[ultima].tonica = true;
        return ultima as i32;
    }

    let nucleo_ultimo: String = silabas[ultima].nucleo.concat();
    let coda_ultima: String = silabas[ultima].coda.concat();
    let terminacao = format!("{}{}", nucleo_ultimo, coda_ultima);

    if eh_oxitona_is_us(palavra) {
        silabas[ultima].tonica = true;
        return ultima as i32;
    }

    if eh_oxitona_om_um(palavra, silabas.len()) {
        silabas[ultima].tonica = true;
        return ultima as i32;
    }

    if eh_oxitona_irdes(palavra) && silabas.len() >= 3 {
        let penultima = ultima - 1;
        let nucleo_penultimo: String = silabas[penultima].nucleo.concat();
        if nucleo_penultimo == "i" || nucleo_penultimo == "í" {
            silabas[penultima].tonica = true;
            return penultima as i32;
        }
    }

    let coda_consonantal =
        matches!(coda_ultima.as_str(), "r" | "l" | "z" | "x" | "n");
    let ditongo_mais_s = coda_ultima == "s" && silabas[ultima].nucleo.len() == 2;
    let nasal_final = (coda_ultima == "m" || coda_ultima == "ns")
        && (nucleo_ultimo == "i" || nucleo_ultimo == "u");
    let iu_final = coda_ultima.is_empty()
        && (nucleo_ultimo == "i" || nucleo_ultimo == "u")
        && nucleo_ultimo.chars().count() == 1;

    let ditongo_final_oxitono = coda_ultima.is_empty()
        && matches!(
            nucleo_ultimo.as_str(),
            "ai" | "ei" | "oi" | "au" | "eu" | "iu" | "ou"
        );

    let oxitona = coda_consonantal
        || ditongo_mais_s
        || nasal_final
        || iu_final
        || ditongo_final_oxitono;

    let paroxitona_forcada =
        matches!(terminacao.as_str(), "em" | "ens" | "am" | "ams");

    if oxitona && !paroxitona_forcada {
        silabas[ultima].tonica = true;
        return ultima as i32;
    }

    let posicao_tonica = silabas.len() - 2;
    silabas[posicao_tonica].tonica = true;
    posicao_tonica as i32
}

fn acento_secundario(silabas: &mut [Silaba], posicao_tonica: i32, palavra: &str) {
    if silabas.len() < 3 || posicao_tonica <= 0 {
        return;
    }
    let tonica = posicao_tonica as usize;

    if tem_prefixo_sobre(palavra, silabas.len()) {
        let mut pos = 2;
        while pos < tonica.saturating_sub(1) {
            silabas[pos].secundaria = true;
            pos += 2;
        }
        return;
    }

    silabas[0].secundaria = true;

    let mut pos = 2;
    while pos < tonica.saturating_sub(1) {
        silabas[pos].secundaria = true;
        pos += 2;
    }
}

/* ------------------------------------------------------------------ *
 * 4. Mapeamento para IPA
 * ------------------------------------------------------------------ */

fn nucleo_nasal(vogal: &str, nasal_por_coda: bool) -> String {
    let caractere = vogal.chars().next().unwrap_or(' ');
    match caractere {
        'a' | 'á' | 'à' | 'â' | 'ã' => "ɐ\u{0303}".to_string(),
        'e' | 'é' | 'ê' => {
            if nasal_por_coda {
                "eɪ".to_string()
            } else {
                "e".to_string()
            }
        },
        'i' | 'í' => "i".to_string(),
        'o' | 'ó' | 'ô' => "o".to_string(),
        'õ' => "o\u{0303}".to_string(),
        'u' | 'ú' => "u\u{0303}".to_string(),
        _ => vogal.to_string(),
    }
}

fn vogal_oral(
    vogal: &str,
    tonica: bool,
    final_palavra: bool,
    pretonica_nasal: bool,
    postonica: bool,
) -> String {
    let caractere = vogal.chars().next().unwrap_or(' ');
    match caractere {
        'a' => {
            if (final_palavra && !tonica) || pretonica_nasal || postonica {
                "æ".to_string()
            } else {
                "a".to_string()
            }
        },
        'á' | 'à' => "a".to_string(),
        'â' => "ɐ".to_string(),
        'ã' => "ɐ\u{0303}".to_string(),
        'e' => {
            if final_palavra && !tonica {
                "y".to_string()
            } else {
                "e".to_string()
            }
        },
        'é' => "ɛ".to_string(),
        'ê' => "e".to_string(),
        'i' | 'í' => "i".to_string(),
        'o' => {
            if final_palavra && !tonica {
                "ʊ".to_string()
            } else {
                "o".to_string()
            }
        },
        'ó' => "ɔ".to_string(),
        'ô' => "o".to_string(),
        'õ' => "õ".to_string(),
        'u' | 'ú' | 'ü' => "u".to_string(),
        _ => vogal.to_string(),
    }
}

fn offglide(vogal: &str, vogal_principal: char, nasal: bool) -> String {
    let caractere = vogal.chars().next().unwrap_or(' ');
    match caractere {
        'i' | 'í' => {
            if nasal {
                "ɪ\u{0303}".to_string()
            } else {
                "ɪ".to_string()
            }
        },
        'u' | 'ú' => {
            if nasal {
                "ʊ\u{0303}".to_string()
            } else {
                match vogal_principal {
                    'a' | 'á' | 'à' | 'â' => "ʊ".to_string(),
                    'e' | 'é' | 'ê' => "ʊ".to_string(),
                    _ => "w".to_string(),
                }
            }
        },
        'o' => {
            if nasal {
                "ʊ\u{0303}".to_string()
            } else {
                "w".to_string()
            }
        },
        'e' => {
            if nasal {
                "ɪ\u{0303}".to_string()
            } else {
                "ɪ".to_string()
            }
        },
        _ => vogal.to_string(),
    }
}

struct ContextoNucleo {
    final_palavra: bool,
    nasal_por_coda: bool,
    nasal_intervoc: bool,
    pretonica_nasal: bool,
    postonica: bool,
}

fn mapear_nucleo(silaba: &Silaba, contexto: &ContextoNucleo) -> String {
    let tonica = silaba.tonica;
    let vogais = &silaba.nucleo;
    if vogais.is_empty() {
        return String::new();
    }

    let tem_til = vogais
        .iter()
        .any(|v| v.chars().next().map_or(false, |c| TIL.contains(c)));
    let nasal = contexto.nasal_por_coda || contexto.nasal_intervoc || tem_til;

    if vogais.len() == 2 && tem_til {
        let primeira = &vogais[0];
        let segunda = vogais[1].chars().next().unwrap_or(' ');
        let base = nucleo_nasal(primeira, contexto.nasal_por_coda);
        if segunda == 'o' || segunda == 'u' {
            return format!("{}ʊ\u{0303}", base);
        }
        if segunda == 'e' || segunda == 'i' {
            return format!("{}ɪ\u{0303}", base);
        }
        return format!("{}{}", base, offglide(&vogais[1], 'ã', true));
    }

    if vogais.len() == 1 {
        let vogal = &vogais[0];
        if nasal {
            return nucleo_nasal(vogal, contexto.nasal_por_coda);
        }
        return vogal_oral(
            vogal,
            tonica,
            contexto.final_palavra,
            contexto.pretonica_nasal,
            contexto.postonica,
        );
    }

    let primeira = &vogais[0];
    let segunda = &vogais[1];
    let primeira_c = primeira.chars().next().unwrap_or(' ');
    let segunda_c = segunda.chars().next().unwrap_or(' ');

    if eh_fraca(primeira_c) && !eh_fraca(segunda_c) {
        let glide = if primeira_c == 'i' || primeira_c == 'í' {
            "j"
        } else {
            "w"
        };

        let segunda_tem_acento = ACENTO_GRAFICO.contains(segunda_c);

        if tonica || segunda_tem_acento {
            let v1 = vogal_oral(primeira, true, false, false, false);
            let v2 = vogal_oral(segunda, false, contexto.final_palavra, false, false);
            return format!("{}{}", v1, v2);
        }

        let v2 = vogal_oral(
            segunda,
            tonica,
            contexto.final_palavra,
            contexto.pretonica_nasal,
            contexto.postonica,
        );
        return format!("{}{}", glide, v2);
    }

    let base = if primeira_c == 'â' {
        "æ".to_string()
    } else if nasal {
        nucleo_nasal(primeira, contexto.nasal_por_coda)
    } else {
        vogal_oral(primeira, tonica, false, contexto.pretonica_nasal, false)
    };
    let nasal_offglide = nasal && primeira_c != 'â';
    format!("{}{}", base, offglide(segunda, primeira_c, nasal_offglide))
}

fn consoante_nasal_coda(proximo_onset: Option<char>) -> &'static str {
    match proximo_onset {
        Some(c) if "pbm".contains(c) => "m",
        _ => "ŋ",
    }
}

struct ContextoOnset<'a> {
    nucleo_ipa: &'a str,
    inicio_palavra: bool,
    coda_anterior: &'a str,
    intervocalico: bool,
    forcar_ks: bool,
}

fn mapear_onset(consoantes: &[String], contexto: &ContextoOnset) -> String {
    let mut saida = String::new();

    for indice in 0..consoantes.len() {
        let consoante = &consoantes[indice];
        let proxima_consoante: Option<&String> = consoantes.get(indice + 1);
        let primeiro = indice == 0;
        let vogal_seguinte = contexto.nucleo_ipa.chars().next().unwrap_or(' ');
        let brando = "iɪeɛy".contains(vogal_seguinte) && proxima_consoante.is_none();

        match consoante.as_str() {
            "ch" => saida.push('ʃ'),
            "lh" => saida.push_str("lj"),
            "nh" => saida.push('ɲ'),
            "rr" => saida.push('x'),
            "ss" => saida.push('s'),
            "qu" => saida.push('k'),
            "gu" => saida.push('ɡ'),
            "kw" => saida.push_str("kw"),
            "gw" => saida.push_str("ɡw"),

            "b" => saida.push('b'),
            "c" => {
                if brando { saida.push('s'); } else { saida.push('k'); }
            },
            "ç" => saida.push('s'),
            "d" => {
                if "iɪ".contains(vogal_seguinte) || vogal_seguinte == 'y' {
                    if proxima_consoante.is_some() {
                        saida.push('d');
                    } else {
                        saida.push_str("dʒ");
                    }
                } else {
                    saida.push('d');
                }
            },
            "f" => saida.push('f'),
            "g" => {
                if brando { saida.push('ʒ'); } else { saida.push('ɡ'); }
            },
            "h" => {},
            "j" => saida.push('ʒ'),
            "k" => saida.push('k'),
            "l" => saida.push('l'),
            "m" => saida.push('m'),
            "n" => saida.push('n'),
            "p" => saida.push('p'),
            "q" => saida.push('k'),
            "r" => {
                let coda_terminal = contexto.coda_anterior.chars().last();
                let coda_consonantal =
                    coda_terminal.map_or(false, |c| "nlsɾŋ".contains(c));
                if primeiro && (contexto.inicio_palavra || coda_consonantal) {
                    saida.push('x');
                } else if primeiro && contexto.coda_anterior.is_empty() {
                    saida.push('ɾ');
                } else {
                    saida.push('r');
                }
            },
            "s" => {
                if contexto.intervocalico && primeiro {
                    saida.push('z');
                } else {
                    saida.push('s');
                }
            },
            "t" => {
                if "iɪ".contains(vogal_seguinte) || vogal_seguinte == 'y' {
                    if proxima_consoante.is_some() {
                        saida.push('t');
                    } else {
                        saida.push_str("tʃ");
                    }
                } else {
                    saida.push('t');
                }
            },
            "v" => saida.push('v'),
            "w" => saida.push('w'),
            "x" => {
                if contexto.forcar_ks {
                    saida.push_str("ks");
                } else {
                    saida.push('ʃ');
                }
            },
            "y" => saida.push('j'),
            "z" => saida.push('z'),
            _ => saida.push_str(consoante),
        }
    }
    saida
}

struct ContextoCoda {
    proximo_onset: Option<char>,
    final_palavra: bool,
    sonorizar_s: bool,
    #[allow(dead_code)]
    silaba_acentuada: bool,
    vogal_principal: char,
}

fn mapear_coda(consoantes: &[String], contexto: &ContextoCoda) -> String {
    let mut saida = String::new();

    for indice in 0..consoantes.len() {
        let consoante = &consoantes[indice];
        let eh_ultima = indice == consoantes.len() - 1;
        let seguinte: Option<char> = if indice + 1 < consoantes.len() {
            consoantes[indice + 1].chars().next()
        } else if eh_ultima {
            contexto.proximo_onset
        } else {
            None
        };

        match consoante.as_str() {
            "m" | "n" => saida.push_str(consoante_nasal_coda(seguinte)),
            "r" => {
                if contexto.final_palavra && eh_ultima {
                    saida.push('r');
                } else {
                    saida.push_str("ɾə");
                }
            },
            "l" => {
                if contexto.final_palavra && eh_ultima {
                    let vogal = contexto.vogal_principal;
                    match vogal {
                        'o' | 'ó' | 'ô' => {
                            saida.push('l');
                        },
                        _ => {
                            saida.push('w');
                        },
                    }
                } else {
                    let vogal = contexto.vogal_principal;
                    match vogal {
                        'a' | 'á' | 'à' | 'â' | 'e' | 'é' | 'ê' => {
                            saida.push('ʊ');
                        },
                        _ => {
                            saida.push('w');
                        },
                    }
                }
            },
            "s" | "ss" => {
                if contexto.final_palavra && eh_ultima {
                    saida.push(if contexto.sonorizar_s { 'z' } else { 's' });
                } else {
                    let proxima = seguinte.unwrap_or(' ');
                    saida.push(if "bdgjlmnrvzç".contains(proxima) { 'z' } else { 's' });
                }
            },
            "z" => {
                if contexto.final_palavra && eh_ultima {
                    saida.push(if contexto.sonorizar_s { 'z' } else { 's' });
                } else {
                    saida.push('z');
                }
            },
            "x" => saida.push('s'),
            "c" => saida.push('k'),
            "ç" => saida.push('s'),
            "b" => saida.push('b'),
            "d" => saida.push('d'),
            "g" => saida.push('ɡ'),
            "p" => saida.push('p'),
            "t" => saida.push_str("tʃ"),
            "ch" => saida.push('ʃ'),
            _ => saida.push_str(consoante),
        }
    }
    saida
}

pub fn palavra_para_ipa(palavra: &str, proxima_inicial: Option<char>) -> String {
    let palavra_minuscula = palavra.to_lowercase();
    if palavra_minuscula.is_empty() {
        return String::new();
    }

    let mut silabas = silabificar(&palavra_minuscula);
    if silabas.is_empty() {
        return String::new();
    }

    let posicao_tonica = acentuar(&mut silabas, &palavra_minuscula);
    acento_secundario(&mut silabas, posicao_tonica, &palavra_minuscula);

    // Sândi é aplicado em `resolver_palavra`, não aqui.
    let sonorizar_s = false;

    let caracteres: Vec<char> = palavra_minuscula.chars().collect();
    let prefixo_ex = caracteres.len() > 2
        && caracteres[0] == 'e'
        && caracteres[1] == 'x'
        && eh_vogal(caracteres[2]);

    let forcar_ks = tem_radical_ks(&palavra_minuscula);

    let mut saida = String::new();
    let mut coda_anterior = String::new();

    for posicao in 0..silabas.len() {
        let silaba_atual = &silabas[posicao];
        let proxima_silaba = silabas.get(posicao + 1);
        let final_palavra = posicao == silabas.len() - 1;

        let postonica = posicao > posicao_tonica as usize;

        let eh_am_final = final_palavra
            && silaba_atual.nucleo.len() == 1
            && silaba_atual.nucleo[0]
                .chars()
                .next()
                .map_or(false, |c| "aáàã".contains(c))
            && silaba_atual.coda.len() == 1
            && silaba_atual.coda[0] == "m";

        let nasal_por_coda = silaba_atual
            .coda
            .first()
            .map_or(false, |c| NASALIZAVEIS.contains(&c.as_str()));

        let proxima_e_nh = proxima_silaba
            .map_or(false, |p| p.onset.len() == 1 && p.onset[0] == "nh");

        let contato_nasal = !nasal_por_coda
            && silaba_atual.coda.is_empty()
            && proxima_silaba.map_or(false, |p| {
                p.onset.len() == 1 && NASALIZAVEIS.contains(&p.onset[0].as_str())
            })
            && silaba_atual.nucleo.len() == 1;

        let vogal_atual = silaba_atual
            .nucleo
            .first()
            .and_then(|v| v.chars().next())
            .unwrap_or(' ');

        let eh_a_ou_acentuada = "aáàâã".contains(vogal_atual);
        let eh_u = "uú".contains(vogal_atual);

        let nasal_intervoc = contato_nasal
            && ((proxima_e_nh && (eh_a_ou_acentuada || eh_u))
                || (!proxima_e_nh
                    && ((eh_a_ou_acentuada && silaba_atual.tonica)
                        || (eh_u && silaba_atual.tonica))));

        let pretonica_nasal = contato_nasal
            && eh_a_ou_acentuada
            && !silaba_atual.tonica
            && !proxima_e_nh;

        let coda_so_s = silaba_atual.coda.len() == 1
            && (silaba_atual.coda[0] == "s" || silaba_atual.coda[0] == "ss");
        let final_palavra_nucleo =
            final_palavra && (silaba_atual.coda.is_empty() || coda_so_s);

        let nucleo_ipa = if eh_am_final {
            "ɐ\u{0303}ʊ\u{0303}".to_string()
        } else {
            mapear_nucleo(
                silaba_atual,
                &ContextoNucleo {
                    final_palavra: final_palavra_nucleo,
                    nasal_por_coda,
                    nasal_intervoc,
                    pretonica_nasal,
                    postonica,
                },
            )
        };

        let vogal_principal = silaba_atual
            .nucleo
            .last()
            .and_then(|v| v.chars().next())
            .unwrap_or(' ');

        let intervocalico = posicao > 0 && silabas[posicao - 1].coda.is_empty();

        let onset_ipa = if prefixo_ex
            && posicao == 1
            && silaba_atual.onset.first().map_or(false, |c| c == "x")
        {
            let mut onset_corrigido = silaba_atual.onset.clone();
            onset_corrigido[0] = "z".to_string();
            mapear_onset(
                &onset_corrigido,
                &ContextoOnset {
                    nucleo_ipa: &nucleo_ipa,
                    inicio_palavra: false,
                    coda_anterior: &coda_anterior,
                    intervocalico,
                    forcar_ks,
                },
            )
        } else {
            mapear_onset(
                &silaba_atual.onset,
                &ContextoOnset {
                    nucleo_ipa: &nucleo_ipa,
                    inicio_palavra: posicao == 0,
                    coda_anterior: &coda_anterior,
                    intervocalico,
                    forcar_ks,
                },
            )
        };

        let coda_ipa = if eh_am_final {
            String::new()
        } else {
            let proximo_onset_char =
                proxima_silaba.and_then(|p| p.onset.join("").chars().next());
            mapear_coda(
                &silaba_atual.coda,
                &ContextoCoda {
                    proximo_onset: proximo_onset_char,
                    final_palavra,
                    sonorizar_s,
                    silaba_acentuada: silaba_atual.tonica || silaba_atual.secundaria,
                    vogal_principal,
                },
            )
        };

        coda_anterior = coda_ipa.clone();

        let marca = if silaba_atual.tonica {
            "ˈ"
        } else if silaba_atual.secundaria {
            "ˌ"
        } else {
            ""
        };
        saida.push_str(&onset_ipa);
        saida.push_str(marca);
        saida.push_str(&nucleo_ipa);
        saida.push_str(&coda_ipa);
    }

    limpar(&saida)
}

static RE_ACENTO_DUPLICADO: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"ˈ{2,}").unwrap());
static RE_TIL_DUPLICADO: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\u{0303}{2,}").unwrap());
static RE_IDEO: Lazy<Regex> = Lazy::new(|| Regex::new(r"id[eɛ]ʊ$").unwrap());
static RE_PALATAL_Y: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(tʃ|dʒ|ʒ)y$").unwrap());
static RE_PALATAL_U: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([ʃʒz])ʊ$").unwrap());

fn limpar(ipa: &str) -> String {
    let decomposto: String = ipa.nfd().collect();
    let sem_g = decomposto.replace('g', "ɡ");
    let sem_acento_duplicado =
        RE_ACENTO_DUPLICADO.replace_all(&sem_g, "ˈ").into_owned();
    let sem_til_duplicado = RE_TIL_DUPLICADO
        .replace_all(&sem_acento_duplicado, "\u{0303}")
        .into_owned();
    let sem_ss = sem_til_duplicado.replace("ss", "s");
    let sem_zs = sem_ss.replace("zs", "s");
    let sem_ideo = RE_IDEO.replace_all(&sem_zs, "idʒjʊ").into_owned();

    // Notação: `y` final após africada palatal → `j` (espeak).
    let sem_palatal_y = RE_PALATAL_Y
        .replace_all(&sem_ideo, "${1}j")
        .into_owned();
    // Notação: `ʊ` final após fricativa palatal → `w` (espeak).
    let sem_palatal_u = RE_PALATAL_U
        .replace_all(&sem_palatal_y, "${1}w")
        .into_owned();

    sem_palatal_u.trim().to_string()
}

/* ------------------------------------------------------------------ *
 * 5. API pública
 * ------------------------------------------------------------------ */

static RE_TOKEN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"[\p{L}\p{M}'-]+|[;:,.!?—…\x22«»“”(){}₹\[\]]|\s+"#).unwrap()
});

static RE_PONTUACAO: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^[;:,.!?¡¿—…\x22«»“”(){}₹\[\]]+$"#).unwrap()
});

static RE_ESPACO: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\s+$").unwrap());
static RE_ESPACOS_MULTIPLOS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s{2,}").unwrap());
static RE_ESPACO_ANTES_PONTUACAO: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s+([;:,.!?…])").unwrap());

pub struct OpcoesFonemizar<'a> {
    pub normalizar: bool,
    pub lexico: Option<&'a HashMap<String, String>>,
}

impl<'a> Default for OpcoesFonemizar<'a> {
    fn default() -> Self {
        Self {
            normalizar: true,
            lexico: None,
        }
    }
}

pub fn fonemizar(texto: &str, opcoes: &OpcoesFonemizar) -> String {
    let texto_processado = if opcoes.normalizar {
        normalizar(texto, OpcoesNormalizar::default())
    } else {
        texto.to_string()
    };
    if texto_processado.is_empty() {
        return String::new();
    }

    let tokens: Vec<&str> = RE_TOKEN
        .find_iter(&texto_processado)
        .map(|m| m.as_str())
        .collect();
    let palavras: Vec<&str> = tokens
        .iter()
        .filter(|x| x.chars().any(char::is_alphabetic))
        .copied()
        .collect();

    let mut partes: Vec<String> = Vec::new();
    let mut indice_palavra = 0;

    for token in &tokens {
        if RE_ESPACO.is_match(token) {
            if partes.last().map_or(false, |p| p != " ") {
                partes.push(" ".to_string());
            }
            continue;
        }

        if RE_PONTUACAO.is_match(token) {
            while partes.last().map_or(false, |p| p == " ") {
                partes.pop();
            }
            partes.push((*token).to_string());
            partes.push(" ".to_string());
            continue;
        }

        let proxima = palavras
            .get(indice_palavra + 1)
            .copied()
            .unwrap_or("");
        indice_palavra += 1;

        let palavra_bruta = token
            .to_lowercase()
            .trim_start_matches(|c| c == '\'' || c == '-')
            .trim_end_matches(|c| c == '\'' || c == '-')
            .to_string();
        if palavra_bruta.is_empty() {
            continue;
        }

        if palavra_bruta.contains('-') {
            let subpalavras: Vec<&str> = palavra_bruta
                .split('-')
                .filter(|w| !w.is_empty())
                .collect();
            let ipa_sub: Vec<String> = subpalavras
                .iter()
                .map(|w| resolver_palavra(w, None, opcoes.lexico))
                .collect();
            partes.push(ipa_sub.join(" "));
            continue;
        }

        let proxima_inicial = proxima.chars().next();
        partes.push(resolver_palavra(
            &palavra_bruta,
            proxima_inicial,
            opcoes.lexico,
        ));
    }

    let junto = partes.join("");
    let sem_espacos_multiplos =
        RE_ESPACOS_MULTIPLOS.replace_all(&junto, " ").into_owned();
    let sem_espaco_antes_pontuacao = RE_ESPACO_ANTES_PONTUACAO
        .replace_all(&sem_espacos_multiplos, "$1")
        .into_owned();

    sem_espaco_antes_pontuacao.trim().to_string()
}

/// Aplica a sonorização do `s` final em contato com a palavra seguinte.
fn aplicar_sandi_s_final(ipa: String, proxima_inicial: Option<char>) -> String {
    let Some(inicial_bruta) = proxima_inicial else {
        return ipa;
    };

    // O `proxima_inicial` vem do texto original, que pode estar em
    // caixa alta (`Os`, `As`). Normaliza para minúscula.
    let inicial = inicial_bruta.to_lowercase().next().unwrap_or(' ');

    if !ipa.ends_with('s') {
        return ipa;
    }

    let sonoriza = eh_vogal(inicial) || "bdgjlmnrvz".contains(inicial);
    if !sonoriza {
        return ipa;
    }

    let mut resultado = ipa;
    resultado.pop();
    resultado.push('z');
    resultado
}

fn resolver_palavra(
    palavra: &str,
    proxima_inicial: Option<char>,
    lexico_extra: Option<&HashMap<String, String>>,
) -> String {
    // 1. Clíticos contextuais têm prioridade máxima.
    let ipa_base = if let Some(forma) = buscar_clitico_contexto(palavra) {
        forma.to_string()
    } else if let Some(mapa) = lexico_extra {
        // 2. Léxico do usuário / auto-carregado.
        if let Some(ipa) = mapa.get(palavra) {
            ipa.clone()
        } else if let Some(lexico) = buscar_lexico(palavra) {
            lexico.to_string()
        } else if let Some(clitico) = buscar_clitico(palavra) {
            clitico.to_string()
        } else {
            palavra_para_ipa(palavra, proxima_inicial)
        }
    } else if let Some(lexico) = buscar_lexico(palavra) {
        lexico.to_string()
    } else if let Some(clitico) = buscar_clitico(palavra) {
        clitico.to_string()
    } else {
        palavra_para_ipa(palavra, proxima_inicial)
    };

    aplicar_sandi_s_final(ipa_base, proxima_inicial)
}

pub fn phonemize(texto: &str, opcoes: &OpcoesFonemizar) -> String {
    fonemizar(texto, opcoes)
}

/* ------------------------------------------------------------------ *
 * Testes
 * ------------------------------------------------------------------ */

#[cfg(test)]
mod testes {
    use super::*;

    fn ipa(palavra: &str) -> String {
        palavra_para_ipa(palavra, None)
    }

    // --- Notação ---

    #[test]
    fn e_final_apos_africada_palatal_vira_j() {
        for (palavra, esperado) in [
            ("existe", "tʃj"),
            ("hoje", "ʒj"),
        ] {
            let r = ipa(palavra);
            assert!(
                r.contains(esperado),
                "{}: esperava '{}', veio {}", palavra, esperado, r
            );
        }
    }

    #[test]
    fn o_final_apos_fricativa_palatal_vira_w() {
        for (palavra, esperado) in [
            ("abaixo", "ʃw"),
            ("criterioso", "zw"),
        ] {
            let r = ipa(palavra);
            assert!(
                r.contains(esperado),
                "{}: esperava '{}', veio {}", palavra, esperado, r
            );
        }
    }

    #[test]
    fn palatal_y_nao_afeta_outras_vogais() {
        let r = ipa("casa");
        assert!(!r.contains("j"), "casa: não esperava 'j', veio {}", r);
    }

    // --- Sândi ---

    #[test]
    fn sandi_s_final_em_sentenca() {
        let r = fonemizar("Muitas vezes Os animais", &OpcoesFonemizar::default());
        assert!(
            r.contains("mwˈiŋtæz vˈezyz ʊz ˌænimˈaɪs")
                || r.contains("mwˈiŋtæz vˈezyz ʊz ˌænimˈaɪz"),
            "esperava 'mwˈiŋtæz vˈezyz ʊz ˌænimˈaɪz' em: {}",
            r
        );
    }

    #[test]
    fn sandi_s_antes_de_consoante_surda() {
        let r = fonemizar("As coisas", &OpcoesFonemizar::default());
        assert!(
            r.contains("as kˈoɪzæs") || r.contains("as kˈoɪzæz"),
            "esperava 'as kˈoɪzæs' em: {}",
            r
        );
    }

    // --- Clíticos contextuais ---

    #[test]
    fn clitico_que_sem_acento_em_contexto() {
        let r = fonemizar("que", &OpcoesFonemizar::default());
        assert!(
            r.contains("ky") && !r.contains("kˈy"),
            "que: esperava 'ky' sem acento, veio {}", r
        );
    }

    #[test]
    fn clitico_na_sem_acento_em_contexto() {
        let r = fonemizar("na", &OpcoesFonemizar::default());
        assert!(
            r.contains("na") && !r.contains("nˈa"),
            "na: esperava 'na' sem acento, veio {}", r
        );
    }

    #[test]
    fn clitico_para_secundario_em_contexto() {
        let r = fonemizar("para", &OpcoesFonemizar::default());
        assert!(
            r.contains("pˌaɾæ") && !r.contains("pˈaɾæ"),
            "para: esperava 'pˌaɾæ', veio {}", r
        );
    }

    #[test]
    fn clitico_ser_sem_acento_em_contexto() {
        let r = fonemizar("ser", &OpcoesFonemizar::default());
        assert!(
            r.contains("seɾ") && !r.contains("sˈer"),
            "ser: esperava 'seɾ', veio {}", r
        );
    }

    #[test]
    fn clitico_de_vira_dʒy() {
        // `de` sempre passa por `CLITICOS_CONTEXTO` (não há distinção
        // isolado/contexto no resolver_palavra).
        let r = fonemizar("de", &OpcoesFonemizar::default());
        assert!(
            r.contains("dʒy"),
            "de: esperava 'dʒy', veio {}", r
        );
    }

    // --- Regra do `x` ---

    #[test]
    fn x_popular_e_sh() {
        for p in [
            "abacaxi", "mexer", "xará", "lixo", "roxo",
            "baixo", "caixa", "peixe", "deixar", "queixo", "eixo",
        ] {
            let r = ipa(p);
            assert!(r.contains('ʃ'), "{}: esperava ʃ, veio {}", p, r);
        }
    }

    #[test]
    fn x_inicial_e_sh() {
        let r = ipa("xícara");
        assert!(r.starts_with('ʃ'), "esperava ʃ no início: {}", r);
    }

    #[test]
    fn radical_taxi_forca_ks() {
        for p in ["táxi", "taxista", "taxímetro", "taxiar"] {
            let r = ipa(p);
            assert!(r.contains("ks"), "{}: esperava ks, veio {}", p, r);
        }
    }

    #[test]
    fn radical_fix_forca_ks() {
        for p in ["fixo", "fixar", "prefixo", "sufixo", "infixo", "crucifixo"] {
            let r = ipa(p);
            assert!(r.contains("ks"), "{}: esperava ks, veio {}", p, r);
        }
    }

    #[test]
    fn radical_sex_forca_ks() {
        for p in ["sexo", "sexual", "sexismo", "assexuado", "bissexual"] {
            let r = ipa(p);
            assert!(r.contains("ks"), "{}: esperava ks, veio {}", p, r);
        }
    }

    #[test]
    fn radical_toxic_forca_ks() {
        for p in ["tóxico", "toxicidade", "toxicologia"] {
            let r = ipa(p);
            assert!(r.contains("ks"), "{}: esperava ks, veio {}", p, r);
        }
    }

    #[test]
    fn radical_nex_forca_ks() {
        for p in ["nexo", "conexo", "desconexo", "anexo", "anexar"] {
            let r = ipa(p);
            assert!(r.contains("ks"), "{}: esperava ks, veio {}", p, r);
        }
    }

    #[test]
    fn radicais_quimica_forca_ks() {
        for p in [
            "hidroxila", "hidróxido", "carboxila", "carboxilase",
            "carboxílico", "oxidação", "oxidante", "oxigênio",
            "dióxido", "peróxido", "hexágono", "hexacampeão",
            "flexão", "flexível", "flexibilizar",
        ] {
            let r = ipa(p);
            assert!(r.contains("ks"), "{}: esperava ks, veio {}", p, r);
        }
    }

    #[test]
    fn hidroxido_com_acento_e_ks() {
        let r = ipa("hidróxido");
        assert!(r.contains("ks"), "hidróxido: esperava ks, veio {}", r);
    }

    #[test]
    fn taxa_nao_vira_ks() {
        for p in ["taxa", "taxar", "taxação"] {
            let r = ipa(p);
            assert!(!r.contains("ks"), "{}: não esperava ks, veio {}", p, r);
        }
    }

    #[test]
    fn sexta_nao_vira_ks() {
        for p in ["sexta", "sexto", "sextante", "sextilha"] {
            let r = ipa(p);
            assert!(!r.contains("ks"), "{}: não esperava ks, veio {}", p, r);
        }
    }

    #[test]
    fn anexim_nao_vira_ks() {
        let r = ipa("anexim");
        assert!(!r.contains("ks"), "anexim: não esperava ks, veio {}", r);
    }

    #[test]
    fn prefixo_ex_vira_z() {
        for p in ["exame", "exemplo", "exato", "exíguo", "exórdio"] {
            let r = palavra_para_ipa(p, None);
            assert!(r.contains("ez"), "{}: esperava ez, veio {}", p, r);
        }
    }

    #[test]
    fn ex_mais_consoante_nao_vira_z() {
        let r = ipa("extensão");
        assert!(!r.contains("ez"), "extensão: não esperava ez, veio {}", r);
    }

    #[test]
    fn am_final_vira_ditongo_nasal() {
        for p in ["falam", "cantam", "estouram", "fizeram"] {
            let r = ipa(p);
            assert!(
                r.contains("ɐ\u{0303}ʊ\u{0303}"),
                "{}: esperava ɐ̃ʊ̃, veio {}", p, r
            );
        }
    }

    #[test]
    fn ao_grafico_forma_ditongo() {
        for p in ["pão", "cão", "irmão", "coração", "razão"] {
            let r = ipa(p);
            assert!(
                r.contains("ɐ\u{0303}ʊ\u{0303}"),
                "{}: esperava ɐ̃ʊ̃, veio {}", p, r
            );
        }
    }

    #[test]
    fn ae_grafico_forma_ditongo() {
        for p in ["mãe", "pães"] {
            let r = ipa(p);
            assert!(
                r.contains("ɐ\u{0303}ɪ\u{0303}"),
                "{}: esperava ɐ̃ɪ̃, veio {}", p, r
            );
        }
    }

    #[test]
    fn oe_grafico_forma_ditongo() {
        let r = ipa("põe");
        assert!(r.contains("o\u{0303}ɪ\u{0303}"), "põe: esperava õɪ̃, veio {}", r);
    }

    #[test]
    fn a_nasaliza_sempre() {
        for (p, esperado) in [
            ("cama", "ɐ\u{0303}"),
            ("ano", "ɐ\u{0303}"),
            ("banho", "ɐ\u{0303}"),
            ("campo", "ɐ\u{0303}"),
        ] {
            let r = ipa(p);
            assert!(r.contains(esperado), "{}: esperava '{}', veio {}", p, esperado, r);
        }
    }

    #[test]
    fn e_nasaliza_so_em_coda() {
        assert!(ipa("tempo").contains("eɪ"), "tempo: esperava eɪ");
        assert!(ipa("vem").contains("eɪ"), "vem: esperava eɪ");
        assert!(ipa("homem").contains("eɪ"), "homem: esperava eɪ");
        assert!(
            !ipa("tenho").contains("eɪ"),
            "tenho: não esperava eɪ, veio {}", ipa("tenho")
        );
    }

    #[test]
    fn o_nunca_nasaliza() {
        for p in ["vison", "ponto", "sonho", "campo"] {
            let r = ipa(p);
            assert!(
                !r.contains("o\u{0303}"),
                "{}: não esperava 'õ', veio {}", p, r
            );
        }
    }

    #[test]
    fn i_nunca_nasaliza() {
        for p in ["vinho", "vison", "ninho", "linho"] {
            let r = ipa(p);
            assert!(
                !r.contains("i\u{0303}"),
                "{}: não esperava 'ĩ', veio {}", p, r
            );
        }
    }

    #[test]
    fn u_nasaliza_antes_de_nh() {
        let r = ipa("testemunhando");
        assert!(r.contains("u\u{0303}"), "testemunhando: esperava ũ, veio {}", r);
    }

    #[test]
    fn u_tonico_antes_de_m_nasaliza() {
        let r = ipa("inhaúma");
        assert!(r.contains("u\u{0303}"), "inhaúma: esperava ũ, veio {}", r);
    }

    #[test]
    fn u_atono_antes_de_m_nao_nasaliza() {
        for p in ["resumia", "sumia", "espiava"] {
            let r = ipa(p);
            assert!(
                !r.contains("u\u{0303}"),
                "{}: não esperava ũ, veio {}", p, r
            );
        }
    }

    #[test]
    fn a_pretonico_antes_de_nasal_vira_ae() {
        let r = ipa("banana");
        assert!(r.contains("æn"), "banana: esperava 'æn', veio {}", r);
    }

    #[test]
    fn a_circunflexo_antes_de_nasal_nasaliza() {
        let r = ipa("alofânico");
        assert!(r.contains("ɐ\u{0303}"), "alofânico: esperava ɐ̃, veio {}", r);
    }

    #[test]
    fn nh_nasaliza_a_pretonico() {
        let r = ipa("banheiro");
        assert!(r.contains("ɐ\u{0303}"), "banheiro: esperava ɐ̃, veio {}", r);
    }

    #[test]
    fn un_antes_de_consoante_nasaliza() {
        let r = ipa("pergunta");
        assert!(r.contains("u\u{0303}"), "pergunta: esperava ũ, veio {}", r);
    }

    #[test]
    fn gu_qu_antes_de_a_o_sao_glide() {
        for (p, esperado) in [
            ("guaxarapo", "ɡw"),
            ("quatro", "kw"),
            ("quando", "kw"),
        ] {
            let r = ipa(p);
            assert!(r.contains(esperado), "{}: esperava '{}', veio {}", p, esperado, r);
        }
    }

    #[test]
    fn gu_antes_de_i_e_glide() {
        let r = ipa("linguiça");
        assert!(r.contains("ɡw"), "linguiça: esperava ɡw, veio {}", r);
    }

    #[test]
    fn qu_antes_de_i_e_k() {
        let r = ipa("quironomídeo");
        assert!(
            r.contains('k') && !r.contains("kw"),
            "quironomídeo: esperava k, veio {}", r
        );
    }

    #[test]
    fn gu_qu_antes_de_e_sao_digrafo() {
        assert!(!ipa("quero").contains("kw"));
        assert!(!ipa("guerra").contains("ɡw"));
    }

    #[test]
    fn gu_antes_de_u_nao_e_digrafo() {
        let r = ipa("pergunta");
        assert!(!r.contains("ɡw"));
    }

    #[test]
    fn ditongo_crescente_sem_acento_e_hiato() {
        let r = ipa("resumia");
        assert!(r.contains("mˈiæ") || r.contains("mˈi.æ"));
    }

    #[test]
    fn ditongo_crescente_com_acento_grafico() {
        let r = ipa("zízia");
        assert!(r.contains("zj"), "zízia: esperava zj, veio {}", r);
    }

    #[test]
    fn ditongo_crescente_historia() {
        let r = ipa("história");
        assert!(r.contains("ɾj"), "história: esperava ɾj, veio {}", r);
    }

    #[test]
    fn ci_antes_de_vogal_e_hiato() {
        let r = ipa("cianeto");
        assert!(!r.contains("sj"), "cianeto: não esperava 'sj', veio {}", r);
    }

    #[test]
    fn gi_antes_de_vogal_e_hiato() {
        let r = ipa("girasol");
        assert!(!r.contains("ʒj"), "girasol: não esperava 'ʒj', veio {}", r);
    }

    #[test]
    fn di_antes_de_vogal_e_hiato() {
        let r = ipa("diádico");
        assert!(!r.contains("dʒj"), "diádico: não esperava 'dʒj', veio {}", r);
    }

    #[test]
    fn hiato_com_acento_na_fraca_nao_forma_ditongo() {
        for p in ["saúde", "ruína", "viúva"] {
            let r = ipa(p);
            assert!(!r.contains("wˈi") && !r.contains("jˈu"));
        }
    }

    #[test]
    fn ideo_final_vira_idʒjʊ() {
        let r = ipa("radionuclídeo");
        assert!(r.contains("idʒjʊ"), "esperava 'idʒjʊ', veio {}", r);
    }

    #[test]
    fn ol_final_preserva_l() {
        for p in ["farol", "anasol", "gaiacol", "rol", "sol"] {
            let r = ipa(p);
            assert!(r.contains('l'), "{}: esperava 'l', veio {}", p, r);
        }
    }

    #[test]
    fn al_final_vira_w() {
        for p in ["sal", "Brasil"] {
            let r = ipa(p);
            assert!(!r.contains('l'), "{}: não esperava 'l', veio {}", p, r);
        }
    }

    #[test]
    fn om_final_e_oxitono() {
        for p in ["crepom", "cupom", "batom"] {
            let r = ipa(p);
            assert!(r.contains("ˈo") || r.contains("ˈo\u{0303}"));
        }
    }

    #[test]
    fn irdes_final_tonica_no_i() {
        for p in ["esvairdes", "destruirdes", "sairdes", "possuirdes"] {
            let r = ipa(p);
            assert!(r.contains("ˈi"), "{}: esperava 'ˈi', veio {}", p, r);
        }
    }

    #[test]
    fn acento_secundario_em_banana() {
        let r = ipa("banana");
        assert!(r.contains("ˌ"));
    }

    #[test]
    fn prefixo_sobre_nao_acentua_inicio() {
        for p in ["sobrenaturalizásseis", "sobreviveríamos"] {
            let r = ipa(p);
            assert!(!r.contains("sˌo"), "{}: não esperava 'sˌo', veio {}", p, r);
        }
    }

    #[test]
    fn au_em_diferentes_contextos() {
        for (p, esperado) in [
            ("pau", "aʊ"),
            ("causa", "aʊ"),
            ("autuasses", "aʊ"),
            ("saudade", "aʊ"),
        ] {
            let r = ipa(p);
            assert!(r.contains(esperado), "{}: esperava '{}', veio {}", p, esperado, r);
        }
    }

    #[test]
    fn eu_em_diferentes_contextos() {
        for (p, esperado) in [
            ("meu", "eʊ"),
            ("vendeu", "eʊ"),
            ("deus", "eʊ"),
            ("neutro", "eʊ"),
            ("neurose", "eʊ"),
        ] {
            let r = ipa(p);
            assert!(r.contains(esperado), "{}: esperava '{}', veio {}", p, esperado, r);
        }
    }

    #[test]
    fn eu_com_acento_e_epsilon_u() {
        let r = ipa("céu");
        assert!(r.contains("ɛʊ"));
    }

    #[test]
    fn ou_em_diferentes_contextos() {
        for (p, esperado) in [
            ("cantou", "ow"),
            ("outro", "ow"),
            ("couro", "ow"),
        ] {
            let r = ipa(p);
            assert!(r.contains(esperado), "{}: esperava '{}', veio {}", p, esperado, r);
        }
    }

    #[test]
    fn is_final_de_verbo_e_oxitono() {
        for p in ["medis", "reagis", "eximis", "impus", "transpus"] {
            let r = ipa(p);
            assert!(r.contains("ˈi") || r.contains("ˈu"));
        }
    }

    #[test]
    fn ss_reduz_para_s() {
        let r = ipa("conscientizado");
        assert!(!r.contains("ss"));
    }

    #[test]
    fn zs_reduz_para_s() {
        let r = ipa("coalesçamos");
        assert!(!r.contains("zs"));
    }

    #[test]
    fn a_postônico_vira_ae() {
        let r = ipa("sacárase");
        assert!(r.contains("ɾæz"));
    }

    #[test]
    fn l_antes_de_consoante_apos_a_em_tonica() {
        for (p, esperado) in [
            ("alto", "aʊ"),
            ("palma", "aʊ"),
            ("caldo", "aʊ"),
        ] {
            let r = ipa(p);
            assert!(r.contains(esperado), "{}: esperava '{}', veio {}", p, esperado, r);
        }
    }

    #[test]
    fn r_apos_coda_nasal_vira_x() {
        let r = ipa("enrola");
        assert!(r.contains("ŋx"), "enrola: esperava 'ŋx', veio {}", r);
    }

    #[test]
    fn d_em_coda_nao_africa() {
        for p in ["adversar", "advogado", "advento"] {
            let r = ipa(p);
            assert!(!r.contains("dʒv"));
        }
    }

    #[test]
    fn circunflexo_a_antes_de_i_vira_ae() {
        let r = ipa("câimbra");
        assert!(r.contains("ˈæ"), "câimbra: esperava 'æ', veio {}", r);
    }

    #[test]
    fn silabificar_casa() {
        let s = silabificar("casa");
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn limpar_normaliza_para_nfd() {
        assert!(limpar("kɐ\u{0303}").contains('\u{0303}'));
    }

    #[test]
    fn fonemizar_texto_simples() {
        let r = fonemizar("bom dia", &OpcoesFonemizar::default());
        assert!(!r.is_empty());
    }
}
