//! Worker persistente: recebe JSON via stdin, devolve JSON via stdout.
//!
//! Protocolo idêntico ao `phonemizer_worker.mjs` do Vozz, mais uma ação
//! extra `process_batch` para processar muitas palavras de uma vez.
//!
//! Ações:
//!
//! ```json
//! {"action":"version"}
//! {"action":"set_lexicon","base_path":"...","global_path":"...","voice_paths":{...}}
//! {"action":"process","text":"...","voice":"idoso","overrides":{...}}
//! {"action":"process_batch","words":["casa","banana",...],"voice":"","overrides":{}}
//! ```
//!
//! Pipeline de `process`:
//!
//! 1. `normalizar(texto)`          (Vozz)
//! 2. `dividir_em_sentencas(norm)` (Vozz)
//! 3. `fonemizar(s, { lexico })`   (Vozz)
//!
//! Pipeline de `process_batch`:
//!
//! 1. Para cada palavra, `precisa_normalizar` → `normalizar` (se preciso).
//! 2. `fonemizar(palavra, { lexico })`.
//! 3. Devolve um mapa `{ palavra: ipa }` e o tempo interno em ms.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::time::Instant;

use vozz_g2p_rs::g2p::{fonemizar, OpcoesFonemizar};
use vozz_g2p_rs::normalize::{normalizar, precisa_normalizar, OpcoesNormalizar};
use vozz_g2p_rs::splitter::dividir_em_sentencas;

const SLOW_THRESHOLD_MS: u128 = 50;

/// Identificador de build. Muda a cada alteração do protocolo.
///
/// Para conferir qual versão do worker está rodando:
///
/// ```bash
/// echo '{"action":"version"}' | ./target/release/phonemizer-worker
/// ```
const BUILD_ID: &str = "2024-11-batch-v2";

// ---------------------------------------------------------------------------
// Estado global
// ---------------------------------------------------------------------------

struct Estado {
    lexicon_base: Arc<HashMap<String, String>>,
    lexicon_voices: HashMap<String, Arc<HashMap<String, String>>>,
    lexicon_cached: HashMap<String, Arc<HashMap<String, String>>>,
}

impl Estado {
    fn novo() -> Self {
        Self {
            lexicon_base: Arc::new(HashMap::new()),
            lexicon_voices: HashMap::new(),
            lexicon_cached: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Formato de resposta
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct RespostaOk {
    ok: bool,
}

#[derive(Serialize)]
struct RespostaErro {
    error: String,
}

#[derive(Serialize)]
struct RespostaProcesso {
    sentences: Vec<Sentenca>,
}

#[derive(Serialize)]
struct Sentenca {
    text: String,
    phonemes: String,
}

#[derive(Serialize)]
struct RespostaBatch {
    phonemes: HashMap<String, String>,
    timing_ms: u128,
    total: usize,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn carregar_json_seguro(path: &Option<String>) -> HashMap<String, String> {
    let Some(p) = path.as_deref() else {
        return HashMap::new();
    };
    match std::fs::read_to_string(p) {
        Ok(conteudo) => match serde_json::from_str::<HashMap<String, String>>(&conteudo) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[lexicon] erro parsing {}: {}", p, e);
                HashMap::new()
            }
        },
        Err(e) => {
            eprintln!("[lexicon] erro lendo {}: {}", p, e);
            HashMap::new()
        }
    }
}

fn rebuild_cache(estado: &mut Estado) {
    estado.lexicon_cached.clear();

    for (voice, voice_lex) in &estado.lexicon_voices {
        let mut merged = (*estado.lexicon_base).clone();
        for (k, v) in voice_lex.iter() {
            merged.insert(k.clone(), v.clone());
        }
        estado
            .lexicon_cached
            .insert(voice.clone(), Arc::new(merged));
    }

    estado
        .lexicon_cached
        .insert("__default__".to_string(), Arc::clone(&estado.lexicon_base));
}

fn get_lexicon_for(
    estado: &Estado,
    voice: &str,
    overrides: &HashMap<String, String>,
) -> Arc<HashMap<String, String>> {
    let cached = estado
        .lexicon_cached
        .get(voice)
        .or_else(|| estado.lexicon_cached.get("__default__"))
        .cloned()
        .unwrap_or_else(|| Arc::clone(&estado.lexicon_base));

    if overrides.is_empty() {
        return cached;
    }

    let mut merged = (*cached).clone();
    for (k, v) in overrides {
        merged.insert(k.clone(), v.clone());
    }
    Arc::new(merged)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

fn handle_set_lexicon(estado: &mut Estado, req: &Value) {
    let t0 = Instant::now();

    let base_path = req
        .get("base_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let base = carregar_json_seguro(&base_path);

    let global_path = req
        .get("global_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let global = carregar_json_seguro(&global_path);

    let t_global = Instant::now();

    let mut lexicon_base = base;
    for (k, v) in global {
        lexicon_base.insert(k, v);
    }
    estado.lexicon_base = Arc::new(lexicon_base);

    estado.lexicon_voices.clear();
    if let Some(voice_paths) = req.get("voice_paths").and_then(|v| v.as_object()) {
        for (voice, path_val) in voice_paths {
            if let Some(path) = path_val.as_str() {
                let voice_lex = carregar_json_seguro(&Some(path.to_string()));
                if !voice_lex.is_empty() {
                    estado
                        .lexicon_voices
                        .insert(voice.clone(), Arc::new(voice_lex));
                }
            }
        }
    }

    rebuild_cache(estado);
    let t_end = Instant::now();

    let n_base = estado.lexicon_base.len();
    let n_voices = estado.lexicon_voices.len();
    let n_voice_entries: usize = estado.lexicon_voices.values().map(|d| d.len()).sum();
    let n_cached = estado.lexicon_cached.len();

    eprintln!(
        "[lexicon] base={} entradas | vozes={} ({} específicas) | cache={} merges",
        n_base, n_voices, n_voice_entries, n_cached
    );
    eprintln!(
        "[lexicon-timing] ler={}ms rebuild_cache={}ms total={}ms",
        (t_global - t0).as_millis(),
        (t_end - t_global).as_millis(),
        (t_end - t0).as_millis()
    );
}

fn handle_process(estado: &Estado, req: &Value) -> RespostaProcesso {
    let t0 = Instant::now();

    let text = req
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let voice = req
        .get("voice")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let overrides: HashMap<String, String> = req
        .get("overrides")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();

    let lexico = get_lexicon_for(estado, &voice, &overrides);
    let t_lex = Instant::now();

    let normalizado = normalizar(&text, OpcoesNormalizar::default());
    let t_normalizar = Instant::now();

    let sentencas = dividir_em_sentencas(&normalizado);
    let t_dividir = Instant::now();

    let mut sentences: Vec<Sentenca> = Vec::with_capacity(sentencas.len());
    for s in &sentencas {
        let ts = Instant::now();
        let opcoes = OpcoesFonemizar {
            normalizar: false,
            lexico: Some(lexico.as_ref()),
        };
        let phon = fonemizar(s, &opcoes);
        let dt = ts.elapsed().as_millis();

        if dt > SLOW_THRESHOLD_MS {
            let preview: String = if s.chars().count() > 40 {
                let mut p: String = s.chars().take(40).collect();
                p.push('…');
                p
            } else {
                s.to_string()
            };
            eprintln!("[slow] fonemizar('{}') = {}ms", preview, dt);
        }

        sentences.push(Sentenca {
            text: s.to_string(),
            phonemes: phon,
        });
    }
    let t_fonemizar = Instant::now();

    eprintln!(
        "[timing] lex={}ms normalizar={}ms dividir={}ms fonemizar={}ms ({}s) total={}ms",
        (t_lex - t0).as_millis(),
        (t_normalizar - t_lex).as_millis(),
        (t_dividir - t_normalizar).as_millis(),
        (t_fonemizar - t_dividir).as_millis(),
        sentencas.len(),
        (t_fonemizar - t0).as_millis()
    );

    RespostaProcesso { sentences }
}

/// Processa um lote de palavras em uma única chamada.
///
/// Devolve `{ phonemes: { palavra: ipa }, timing_ms, total }`.
/// O `timing_ms` mede apenas o loop de normalizar + fonemizar, sem
/// incluir o parse da requisição nem a serialização da resposta.
///
/// Otimização: para palavras puramente alfabéticas que não são
/// abreviações conhecidas, o normalizador é pulado. Nesses casos,
/// `normalizar` não faria nenhuma substituição, então o resultado
/// é idêntico e o custo cai de ~15µs para ~1-2µs por palavra.
fn handle_process_batch(estado: &Estado, requisicao: &Value) -> RespostaBatch {
    // -----------------------------------------------------------------------
    // 1. Extrai a voz da requisição (vazio = voz padrão).
    // -----------------------------------------------------------------------
    let voz = requisicao
        .get("voice")
        .and_then(|valor| valor.as_str())
        .unwrap_or("")
        .to_string();

    // -----------------------------------------------------------------------
    // 2. Extrai os overrides de léxico, se houver.
    // -----------------------------------------------------------------------
    let overrides: HashMap<String, String> = requisicao
        .get("overrides")
        .and_then(|valor| valor.as_object())
        .map(|objeto| {
            objeto
                .iter()
                .filter_map(|(chave, valor)| {
                    valor.as_str().map(|texto| (chave.clone(), texto.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    // -----------------------------------------------------------------------
    // 3. Resolve o léxico final (cache + overrides) para a voz pedida.
    // -----------------------------------------------------------------------
    let lexico = get_lexicon_for(estado, &voz, &overrides);

    // -----------------------------------------------------------------------
    // 4. Extrai a lista de palavras da requisição.
    // -----------------------------------------------------------------------
    let palavras: Vec<String> = requisicao
        .get("words")
        .and_then(|valor| valor.as_array())
        .map(|array| {
            array
                .iter()
                .filter_map(|valor| valor.as_str().map(|texto| texto.to_string()))
                .collect()
        })
        .unwrap_or_default();

    let total = palavras.len();
    let mut fonemas: HashMap<String, String> = HashMap::with_capacity(total);

    // -----------------------------------------------------------------------
    // 5. Loop principal: normaliza (se necessário) e fonemiza cada palavra.
    // -----------------------------------------------------------------------
    let inicio_loop = Instant::now();

    for palavra in &palavras {
        // Decide se o normalizador precisa rodar para esta palavra.
        // Palavras puramente alfabéticas que não são abreviações conhecidas
        // passam direto, porque o normalizador não faria nenhuma substituição.
        let entrada = if precisa_normalizar(palavra) {
            normalizar(palavra, OpcoesNormalizar::default())
        } else {
            palavra.clone()
        };

        // O texto já foi normalizado acima (ou é alfabético puro), então
        // desligamos a normalização interna do fonemizador para evitar
        // rodar os regex duas vezes.
        let opcoes = OpcoesFonemizar {
            normalizar: false,
            lexico: Some(lexico.as_ref()),
        };

        let fonema = fonemizar(&entrada, &opcoes);

        // Guarda o resultado usando a palavra original como chave,
        // para o chamador poder correlacionar entrada e saída.
        fonemas.insert(palavra.clone(), fonema);
    }

    let tempo_decorrido_ms = inicio_loop.elapsed().as_millis();

    // -----------------------------------------------------------------------
    // 6. Log de tempo em stderr (não polui o stdout com o JSON de resposta).
    // -----------------------------------------------------------------------
    eprintln!(
        "[batch] {} palavras | loop={}ms | {:.1}us/palavra",
        total,
        tempo_decorrido_ms,
        if total > 0 {
            (tempo_decorrido_ms as f64 * 1000.0) / total as f64
        } else {
            0.0
        }
    );

    // -----------------------------------------------------------------------
    // 7. Devolve a resposta estruturada.
    // -----------------------------------------------------------------------
    RespostaBatch {
        phonemes: fonemas,
        timing_ms: tempo_decorrido_ms,
        total,
    }
}

// ---------------------------------------------------------------------------
// Loop principal
// ---------------------------------------------------------------------------

fn main() {
    let mut estado = Estado::novo();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout_lock = stdout.lock();

    for line_res in stdin.lock().lines() {
        let line = match line_res {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[worker] erro lendo stdin: {}", e);
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let resp = RespostaErro {
                    error: format!("JSON parse: {} (build {})", e, BUILD_ID),
                };
                let json = serde_json::to_string(&resp)
                    .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".to_string());
                let _ = writeln!(stdout_lock, "{}", json);
                let _ = stdout_lock.flush();
                continue;
            }
        };

        let action = req.get("action").and_then(|v| v.as_str()).unwrap_or("");

        let resposta: String = match action {
            "version" => {
                serde_json::to_string(&serde_json::json!({
                    "build": BUILD_ID,
                    "features": ["set_lexicon", "process", "process_batch"],
                }))
                .unwrap_or_else(|_| "{\"build\":\"unknown\"}".to_string())
            }
            "set_lexicon" => {
                handle_set_lexicon(&mut estado, &req);
                serde_json::to_string(&RespostaOk { ok: true })
                    .unwrap_or_else(|_| "{\"ok\":true}".to_string())
            }
            "process" => {
                let r = handle_process(&estado, &req);
                serde_json::to_string(&r)
                    .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".to_string())
            }
            "process_batch" => {
                let r = handle_process_batch(&estado, &req);
                serde_json::to_string(&r)
                    .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".to_string())
            }
            other => {
                let r = RespostaErro {
                    error: format!("Unknown action: {} (build {})", other, BUILD_ID),
                };
                serde_json::to_string(&r)
                    .unwrap_or_else(|_| "{\"error\":\"serialization failed\"}".to_string())
            }
        };

        let _ = writeln!(stdout_lock, "{}", resposta);
        let _ = stdout_lock.flush();
    }
}