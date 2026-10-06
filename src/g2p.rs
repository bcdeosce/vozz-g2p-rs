//! Conversor grafema→fonema pt-BR.
//!
//! Porte de `index.js` do Vozz, com correções alinhadas ao espeak-ng pt-br.
//!
//! Ordem de decisão em modo contexto (2+ palavras):
//!
//! 0. Sentido anotado pelo BCDE-tagger → `lexicon_homografos.json`
//! 1. `buscar_clitico_contexto` (`data/lexicon_contexto.json`)
//! 2. `buscar_lexico_contexto` (`data/lexicon_contexto.json`)
//! 3. `lexicon_espeak_contexto.json`
//! 4. `lexicon_espeak.json`
//! 5. `buscar_lexico` / `buscar_clitico` (`data/lexicon_palavra.json`)
//! 6. `palavra_para_ipa` (regras)
//!
//! Em modo isolado (1 palavra): `buscar_lexico` → `buscar_clitico` →
//! `lexicon_espeak.json` → `palavra_para_ipa`.

use crate::lexicon_contexto::{buscar_clitico_contexto, buscar_lexico_contexto};
use crate::lexicon_homografos::LexiconHomografos;
use crate::lexicon_palavra::{buscar_clitico, buscar_lexico};
use crate::normalize::{normalizar, OpcoesNormalizar};
use crate::tagger::{self, Tagger};
use crate::trema;
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

const GLIDE_FORCAR: &[&str] = &[
    "de", "se", "me", "tive", "onde", "disso", "adicionado",
];

const GLIDE_BLOQUEAR: &[&str] = &["que"];

fn ajuste_especifico(palavra: &str, ipa: String, proxima_inicial: Option<char>) -> String {
    let proxima_eh_vogal = proxima_inicial
        .map(|c| c.to_lowercase().next().map_or(false, eh_vogal))
        .unwrap_or(false);

    match palavra {
        "o" => {
            if proxima_eh_vogal && ipa == "ʊ" {
                return "u".to_string();
            }
            ipa
        }
        "pode" => {
            if !proxima_eh_vogal && ipa.ends_with('j') {
                let mut r = ipa;
                r.pop();
                r.push('y');
                return r;
            }
            ipa
        }
        _ => ipa,
    }
}

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

    // Computa uma vez por palavra. O próprio `u_pronunciado` já
    // retorna cedo se a palavra não contém `q` nem `g`.
    let u_pronunciado = trema::u_pronunciado(palavra);

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

        // `gu`:
        //   - antes de a/o: sempre glide → gw
        //   - antes de e/i: decide pelo trema hipotético
        //   - antes de u:  não é dígrafo (ex.: "pergunta")
        if par == "gu" {
            match proximo_apos_par {
                Some('a') | Some('o') => {
                    unidades.push(Unidade {
                        tipo: TipoUnidade::Consoante,
                        texto: "gw".to_string(),
                    });
                    indice += 2;
                    continue;
                }
                Some('e') | Some('é') | Some('ê')
                | Some('i') | Some('í') => {
                    let texto = if u_pronunciado { "gw" } else { "ɡ" };
                    unidades.push(Unidade {
                        tipo: TipoUnidade::Consoante,
                        texto: texto.to_string(),
                    });
                    indice += 2;
                    continue;
                }
                _ => {}
            }
        }

        // `qu`:
        //   - antes de a/o: sempre glide → kw
        //   - antes de e/i: decide pelo trema hipotético
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
                Some('e') | Some('é') | Some('ê')
                | Some('i') | Some('í') => {
                    let texto = if u_pronunciado { "kw" } else { "k" };
                    unidades.push(Unidade {
                        tipo: TipoUnidade::Consoante,
                        texto: texto.to_string(),
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

pub fn palavra_para_ipa(palavra: &str, _proxima_inicial: Option<char>) -> String {
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

    let sem_palatal_y = RE_PALATAL_Y
        .replace_all(&sem_ideo, "${1}j")
        .into_owned();
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
    pub lexico_contexto: Option<&'a HashMap<String, String>>,
    /// Mapa `(palavra, sentido) → IPA`.
    pub homografos: Option<&'a LexiconHomografos>,
    /// Tagger BCDE. Em modo isolado é ignorado.
    pub tagger: Option<&'a Tagger>,
}

impl<'a> Default for OpcoesFonemizar<'a> {
    fn default() -> Self {
        Self {
            normalizar: true,
            lexico: None,
            lexico_contexto: None,
            homografos: None,
            tagger: None,
        }
    }
}

fn resolver_palavra_isolada(
    palavra: &str,
    _proxima_inicial: Option<char>,
    lexico: Option<&HashMap<String, String>>,
) -> String {
    if let Some(lexico_form) = buscar_lexico(palavra) {
        return lexico_form.to_string();
    }
    if let Some(clitico_form) = buscar_clitico(palavra) {
        return clitico_form.to_string();
    }
    if let Some(mapa) = lexico {
        if let Some(ipa) = mapa.get(palavra) {
            return ipa.clone();
        }
    }
    palavra_para_ipa(palavra, None)
}

fn resolver_palavra_contexto(
    palavra: &str,
    proxima_inicial: Option<char>,
    lexico: Option<&HashMap<String, String>>,
    lexico_contexto: Option<&HashMap<String, String>>,
    homografos: Option<&LexiconHomografos>,
    sentido: Option<&str>,
) -> String {
    // 0. Homógrafo anotado pelo BCDE-tagger. IPA é a forma base;
    //    aplicar todos os sândis.
    if let (Some(hom), Some(s)) = (homografos, sentido) {
        if let Some(ipa) = hom.ipa_para(palavra, s) {
            let ipa_s = aplicar_sandi_s_final(ipa.to_string(), proxima_inicial);
            let ipa_r = aplicar_sandi_rotico(ipa_s, proxima_inicial);
            return aplicar_sandi_glide(ipa_r, proxima_inicial);
        }
    }

    let (ipa_base, veio_do_contexto) =
        if let Some(forma) = buscar_clitico_contexto(palavra) {
            (forma.to_string(), true)
        } else if let Some(forma) = buscar_lexico_contexto(palavra) {
            (forma.to_string(), true)
        } else if let Some(mapa) = lexico_contexto {
            if let Some(ipa) = mapa.get(palavra) {
                (ipa.clone(), true)
            } else if let Some(mapa2) = lexico {
                if let Some(ipa2) = mapa2.get(palavra) {
                    (ipa2.clone(), false)
                } else if let Some(l) = buscar_lexico(palavra) {
                    (l.to_string(), true)
                } else if let Some(c) = buscar_clitico(palavra) {
                    (c.to_string(), true)
                } else {
                    (palavra_para_ipa(palavra, proxima_inicial), false)
                }
            } else if let Some(l) = buscar_lexico(palavra) {
                (l.to_string(), true)
            } else if let Some(c) = buscar_clitico(palavra) {
                (c.to_string(), true)
            } else {
                (palavra_para_ipa(palavra, proxima_inicial), false)
            }
        } else if let Some(mapa) = lexico {
            if let Some(ipa) = mapa.get(palavra) {
                (ipa.clone(), false)
            } else if let Some(l) = buscar_lexico(palavra) {
                (l.to_string(), true)
            } else if let Some(c) = buscar_clitico(palavra) {
                (c.to_string(), true)
            } else {
                (palavra_para_ipa(palavra, proxima_inicial), false)
            }
        } else if let Some(l) = buscar_lexico(palavra) {
            (l.to_string(), true)
        } else if let Some(c) = buscar_clitico(palavra) {
            (c.to_string(), true)
        } else {
            (palavra_para_ipa(palavra, proxima_inicial), false)
        };

    let ipa_com_s = aplicar_sandi_s_final(ipa_base, proxima_inicial);
    let ipa_com_r = aplicar_sandi_rotico(ipa_com_s, proxima_inicial);

    let aplicar_glide = if GLIDE_BLOQUEAR.contains(&palavra) {
        false
    } else if GLIDE_FORCAR.contains(&palavra) {
        true
    } else {
        !veio_do_contexto
    };

    let ipa_com_glide = if aplicar_glide {
        aplicar_sandi_glide(ipa_com_r, proxima_inicial)
    } else {
        ipa_com_r
    };

    ajuste_especifico(palavra, ipa_com_glide, proxima_inicial)
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

    let n_palavras = tokens
        .iter()
        .filter(|t| t.chars().any(char::is_alphabetic) && !RE_PONTUACAO.is_match(t))
        .count();
    let modo_isolado = n_palavras <= 1;

    let palavras: Vec<&str> = tokens
        .iter()
        .filter(|x| x.chars().any(char::is_alphabetic) && !RE_PONTUACAO.is_match(x))
        .copied()
        .collect();

    let proxima_por_palavra: Vec<Option<char>> = {
        let mut posicoes: Vec<usize> = Vec::with_capacity(palavras.len());
        for (i, t) in tokens.iter().enumerate() {
            if t.chars().any(char::is_alphabetic) && !RE_PONTUACAO.is_match(t) {
                posicoes.push(i);
            }
        }

        let mut resultado: Vec<Option<char>> = vec![None; palavras.len()];
        for k in 0..posicoes.len() {
            let pos_atual = posicoes[k];
            let Some(&pos_prox) = posicoes.get(k + 1) else {
                resultado[k] = None;
                continue;
            };

            let tem_pontuacao = ((pos_atual + 1)..pos_prox)
                .any(|j| RE_PONTUACAO.is_match(tokens[j]));

            resultado[k] = if tem_pontuacao {
                None
            } else {
                tokens[pos_prox].chars().next()
            };
        }
        resultado
    };

    // PASSE 1 — anotação de sentido via BCDE-tagger.
    //
    // O tagger recebe a sentença inteira e devolve tokens com `sense`.
    // Mapeamos cada palavra tokenizada pelo g2p para o `sense`
    // correspondente, preservando ordem de aparição para casos de
    // repetição (ex.: duas ocorrências de "sede").
    let anotacoes: Vec<Option<String>> = if let (Some(tagger), false) =
        (opcoes.tagger, modo_isolado)
    {
        let tokens_tagger = tagger::anotar(tagger, &texto_processado);

        let mut por_palavra: HashMap<String, Vec<String>> = HashMap::new();
        for t in &tokens_tagger {
            if let Some(sense) = &t.sense {
                por_palavra
                    .entry(t.word.to_lowercase())
                    .or_default()
                    .push(sense.clone());
            }
        }
        let mut contador: HashMap<String, usize> = HashMap::new();

        palavras
            .iter()
            .map(|p| {
                let base = p.to_lowercase();
                let chave = base
                    .split('-')
                    .next()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&base)
                    .to_string();
                let idx = contador.entry(chave.clone()).or_insert(0);
                let sentido = por_palavra
                    .get(&chave)
                    .and_then(|v| v.get(*idx))
                    .cloned();
                *idx += 1;
                sentido
            })
            .collect()
    } else {
        vec![None; palavras.len()]
    };

    // PASSE 2 — fonemização.
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

        let proxima_inicial = proxima_por_palavra
            .get(indice_palavra)
            .copied()
            .flatten();
        let sentido = anotacoes.get(indice_palavra).and_then(|s| s.as_deref());
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
                .enumerate()
                .map(|(k, w)| {
                    let sub_sentido = if k == 0 { sentido } else { None };
                    if modo_isolado {
                        resolver_palavra_isolada(w, None, opcoes.lexico)
                    } else {
                        resolver_palavra_contexto(
                            w,
                            None,
                            opcoes.lexico,
                            opcoes.lexico_contexto,
                            opcoes.homografos,
                            sub_sentido,
                        )
                    }
                })
                .collect();
            partes.push(ipa_sub.join(" "));
            continue;
        }

        if modo_isolado {
            partes.push(resolver_palavra_isolada(
                &palavra_bruta,
                proxima_inicial,
                opcoes.lexico,
            ));
        } else {
            partes.push(resolver_palavra_contexto(
                &palavra_bruta,
                proxima_inicial,
                opcoes.lexico,
                opcoes.lexico_contexto,
                opcoes.homografos,
                sentido,
            ));
        }
    }

    let junto = partes.join("");
    let sem_espacos_multiplos =
        RE_ESPACOS_MULTIPLOS.replace_all(&junto, " ").into_owned();
    let sem_espaco_antes_pontuacao = RE_ESPACO_ANTES_PONTUACAO
        .replace_all(&sem_espacos_multiplos, "$1")
        .into_owned();

    sem_espaco_antes_pontuacao.trim().to_string()
}

fn aplicar_sandi_s_final(ipa: String, proxima_inicial: Option<char>) -> String {
    if ipa.is_empty() {
        return ipa;
    }

    let Some(inicial_bruta) = proxima_inicial else {
        if ipa.ends_with('z') {
            let mut r = ipa;
            r.pop();
            r.push('s');
            return r;
        }
        return ipa;
    };

    let inicial = inicial_bruta.to_lowercase().next().unwrap_or(' ');
    let sonora = eh_vogal(inicial) || "bdgjlmnrvz".contains(inicial);

    if ipa.ends_with('s') && sonora {
        let mut r = ipa;
        r.pop();
        r.push('z');
        return r;
    }
    if ipa.ends_with('z') && !sonora {
        let mut r = ipa;
        r.pop();
        r.push('s');
        return r;
    }
    ipa
}

fn aplicar_sandi_rotico(ipa: String, proxima_inicial: Option<char>) -> String {
    if ipa.is_empty() {
        return ipa;
    }

    let proxima_eh_vogal = proxima_inicial
        .map(|c| c.to_lowercase().next().map_or(false, eh_vogal))
        .unwrap_or(false);

    if proxima_eh_vogal && ipa.ends_with('r') {
        let mut r = ipa;
        r.pop();
        r.push('ɾ');
        return r;
    }
    if !proxima_eh_vogal && ipa.ends_with('ɾ') {
        let mut r = ipa;
        r.pop();
        r.push('r');
        return r;
    }
    ipa
}

fn aplicar_sandi_glide(ipa: String, proxima_inicial: Option<char>) -> String {
    if ipa.is_empty() {
        return ipa;
    }

    let proxima_eh_vogal = proxima_inicial
        .map(|c| c.to_lowercase().next().map_or(false, eh_vogal))
        .unwrap_or(false);

    if proxima_eh_vogal {
        if ipa.ends_with('ʊ') {
            let mut r = ipa;
            r.pop();
            r.push('w');
            return r;
        }
        if ipa.ends_with('y') {
            let mut r = ipa;
            r.pop();
            r.push('j');
            return r;
        }
    } else {
        if ipa.ends_with('w') {
            let mut r = ipa;
            r.pop();
            r.push('ʊ');
            return r;
        }
        if ipa.ends_with('j') {
            let mut r = ipa;
            r.pop();
            r.push('y');
            return r;
        }
    }
    ipa
}

pub fn phonemize(texto: &str, opcoes: &OpcoesFonemizar) -> String {
    fonemizar(texto, opcoes)
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ipa(palavra: &str) -> String {
        palavra_para_ipa(palavra, None)
    }

    #[test]
    fn cinquentenao_perde_u() {
        let r = ipa("cinquenta");
        assert!(r.contains("kw"), "cinquenta: esperava kw, veio {}", r);
    }

    #[test]
    fn tranquilo_tem_kw() {
        let r = ipa("tranquilo");
        assert!(r.contains("kw"), "tranquilo: esperava kw, veio {}", r);
    }

    #[test]
    fn quente_nao_tem_kw() {
        let r = ipa("quente");
        assert!(!r.contains("kw"), "quente: não esperava kw, veio {}", r);
    }

    #[test]
    fn guerra_nao_tem_gw() {
        let r = ipa("guerra");
        assert!(!r.contains("ɡw"), "guerra: não esperava ɡw, veio {}", r);
    }

    #[test]
    fn linguica_tem_gw() {
        let r = ipa("linguiça");
        assert!(r.contains("ɡw"), "linguiça: esperava ɡw, veio {}", r);
    }

    #[test]
    fn quatro_tem_kw() {
        let r = ipa("quatro");
        assert!(r.contains("kw"), "quatro: esperava kw, veio {}", r);
    }

    #[test]
    fn fonemizar_texto_simples() {
        let r = fonemizar("bom dia", &OpcoesFonemizar::default());
        assert!(!r.is_empty());
    }
}
