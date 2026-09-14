//! Worker persistente: recebe JSON via stdin, devolve JSON via stdout.
//!
//! Protocolo idêntico ao `phonemizer_worker.mjs` do Vozz, mais uma ação
//! extra `process_batch` para processar muitas palavras de uma vez.
//!
//! Ações:
//!
//! ```json
//! {"action":"version"}
//! {"action":"set_lexicon","base_path":"...","global_path":"...","voice_paths":{...},"context_path":"..."}
//! {"action":"process","text":"...","voice":"idoso","overrides":{...}}
//! {"action":"process_batch","words":["casa","banana",...],"voice":"","overrides":{}}
//! ```
//!
//! Pipeline de `process`:
//!
//! 1. `normalizar(texto)`          (Vozz)
//! 2. `dividir_em_sentencas(norm)` (Vozz)
//! 3. `fonemizar(s, { lexico, lexico_contexto, homografos })` (Vozz)
//!
//! Pipeline de `process_batch`:
//!
//! 1. Para cada palavra, `precisa_normalizar` → `normalizar` (se preciso).
//! 2. `fonemizar(palavra, { lexico })`.
//! 3. Devolve um mapa `{ palavra: ipa }` e o tempo interno em ms.
//!
//! Léxicos automáticos:
//!
//! Na inicialização, o worker procura:
//!
//! - `lexico_espeak.json`          → base (modo isolado)
//! - `lexico_espeak_contexto.json` → contexto (modo sentença)
//! - `homograph_rules_v2.json`     → regras de desambiguação
//! - `lexicon_homografos.json`     → mapa (palavra, sentido) → IPA
//!
//! Todos em caminhos comuns (variável de ambiente, pasta do executável,
//! diretório atual, `data/`, `cache/`).

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use vozz_g2p_rs::g2p::{fonemizar, OpcoesFonemizar};
use vozz_g2p_rs::homografos::{Homografos, Regra};
use vozz_g2p_rs::normalize::{normalizar, precisa_normalizar, OpcoesNormalizar};
use vozz_g2p_rs::splitter::dividir_em_sentencas;

const SLOW_THRESHOLD_MS: u128 = 50;

/// Identificador de build. Muda a cada alteração do protocolo.
const BUILD_ID: &str = "2024-11-batch-v5-homografos";

// ---------------------------------------------------------------------------
// Estado global
// ---------------------------------------------------------------------------

struct Estado {
    /// Léxico base (modo isolado).
    lexicon_base: Arc<HashMap<String, String>>,
    /// Léxico contextual (modo sentença).
    lexicon_base_contexto: Arc<HashMap<String, String>>,
    /// Overrides por voz (modo isolado).
    lexicon_voices: HashMap<String, Arc<HashMap<String, String>>>,
    /// Overrides por voz (modo contexto).
    lexicon_voices_contexto: HashMap<String, Arc<HashMap<String, String>>>,
    /// Cache: base + override de voz (modo isolado).
    lexicon_cached: HashMap<String, Arc<HashMap<String, String>>>,
    /// Cache: base + override de voz (modo contexto).
    lexicon_cached_contexto: HashMap<String, Arc<HashMap<String, String>>>,
    /// Desambiguador de homógrafos.
    homografos: Option<Homografos>,
}

impl Estado {
    fn novo() -> Self {
        Self {
            lexicon_base: Arc::new(HashMap::new()),
            lexicon_base_contexto: Arc::new(HashMap::new()),
            lexicon_voices: HashMap::new(),
            lexicon_voices_contexto: HashMap::new(),
            lexicon_cached: HashMap::new(),
            lexicon_cached_contexto: HashMap::new(),
            homografos: None,
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
    estado.lexicon_cached_contexto.clear();

    for (voice, voice_lex) in &estado.lexicon_voices {
        let mut merged = (*estado.lexicon_base).clone();
        for (k, v) in voice_lex.iter() {
            merged.insert(k.clone(), v.clone());
        }
        estado
            .lexicon_cached
            .insert(voice.clone(), Arc::new(merged));
    }

    for (voice, voice_lex) in &estado.lexicon_voices_contexto {
        let mut merged = (*estado.lexicon_base_contexto).clone();
        for (k, v) in voice_lex.iter() {
            merged.insert(k.clone(), v.clone());
        }
        estado
            .lexicon_cached_contexto
            .insert(voice.clone(), Arc::new(merged));
    }

    estado
        .lexicon_cached
        .insert("__default__".to_string(), Arc::clone(&estado.lexicon_base));
    estado.lexicon_cached_contexto.insert(
        "__default__".to_string(),
        Arc::clone(&estado.lexicon_base_contexto),
    );
}

/// Devolve `(lexico_isolado, lexico_contexto)` para a voz/overrides dados.
fn get_lexicons_for(
    estado: &Estado,
    voice: &str,
    overrides: &HashMap<String, String>,
) -> (Arc<HashMap<String, String>>, Arc<HashMap<String, String>>) {
    let cached_iso = estado
        .lexicon_cached
        .get(voice)
        .or_else(|| estado.lexicon_cached.get("__default__"))
        .cloned()
        .unwrap_or_else(|| Arc::clone(&estado.lexicon_base));

    let cached_ctx = estado
        .lexicon_cached_contexto
        .get(voice)
        .or_else(|| estado.lexicon_cached_contexto.get("__default__"))
        .cloned()
        .unwrap_or_else(|| Arc::clone(&estado.lexicon_base_contexto));

    if overrides.is_empty() {
        return (cached_iso, cached_ctx);
    }

    let mut merged_iso = (*cached_iso).clone();
    let mut merged_ctx = (*cached_ctx).clone();
    for (k, v) in overrides {
        merged_iso.insert(k.clone(), v.clone());
        merged_ctx.insert(k.clone(), v.clone());
    }

    (Arc::new(merged_iso), Arc::new(merged_ctx))
}

/// Caminhos padrão onde procurar um arquivo com o nome dado.
/// A variável de ambiente é derivada do nome em maiúsculas.
fn caminhos_arquivo(nome: &str) -> Vec<PathBuf> {
    let mut caminhos = Vec::new();

    let stem = nome.trim_end_matches(".json").to_uppercase();
    let var = format!("VOZZ_{}", stem);
    if let Ok(caminho) = std::env::var(&var) {
        if !caminho.is_empty() {
            caminhos.push(PathBuf::from(caminho));
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            caminhos.push(dir.join(nome));
        }
    }

    caminhos.push(PathBuf::from(nome));
    caminhos.push(PathBuf::from("data").join(nome));
    caminhos.push(PathBuf::from("cache").join(nome));

    caminhos
}

/// Tenta carregar um arquivo. Devolve o caminho carregado e o mapa.
fn carregar_arquivo(nome: &str) -> Option<(PathBuf, HashMap<String, String>)> {
    for caminho in caminhos_arquivo(nome) {
        if !caminho.is_file() {
            continue;
        }
        match std::fs::read_to_string(&caminho) {
            Ok(conteudo) => match serde_json::from_str::<HashMap<String, String>>(&conteudo) {
                Ok(mapa) if !mapa.is_empty() => {
                    return Some((caminho, mapa));
                }
                _ => continue,
            },
            Err(_) => continue,
        }
    }
    None
}

/// Carrega as regras de homógrafos + o léxico `(palavra, sentido) → IPA`.
fn carregar_homografos() -> Option<Homografos> {
    // Localiza `homograph_rules_v2.json`
    let regras_path = caminhos_arquivo("homograph_rules_v2.json")
        .into_iter()
        .find(|p| p.is_file())?;

    // Localiza `lexicon_homografos.json`
    let lexico_path = caminhos_arquivo("lexicon_homografos.json")
        .into_iter()
        .find(|p| p.is_file())?;

    let regras_txt = std::fs::read_to_string(&regras_path).ok()?;
    let lexico_txt = std::fs::read_to_string(&lexico_path).ok()?;

    let regras: HashMap<String, Regra> =
        serde_json::from_str(&regras_txt).ok()?;

    // Formato: { palavra: { sentido: { ipa, confianca, ocorrencias } } }
    #[derive(serde::Deserialize)]
    struct EntradaSentido {
        ipa: String,
        #[serde(default)]
        #[allow(dead_code)]
        confianca: f64,
        #[serde(default)]
        #[allow(dead_code)]
        ocorrencias: usize,
    }
    let lexico_completo: HashMap<String, HashMap<String, EntradaSentido>> =
        serde_json::from_str(&lexico_txt).ok()?;

    // Achata para `HashMap<String, HashMap<String, String>>`
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

    eprintln!(
        "[homografos] regras: {} | lexicon: {} palavras | arquivo regras: {} | arquivo lexico: {}",
        regras.len(),
        lexico.len(),
        regras_path.display(),
        lexico_path.display(),
    );

    Some(Homografos::novo(regras, lexico))
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

    let context_path = req
        .get("context_path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let contexto = carregar_json_seguro(&context_path);

    let t_global = Instant::now();

    let mut lexicon_base = base;
    for (k, v) in global {
        lexicon_base.insert(k, v);
    }
    estado.lexicon_base = Arc::new(lexicon_base);
    estado.lexicon_base_contexto = Arc::new(contexto);

    estado.lexicon_voices.clear();
    estado.lexicon_voices_contexto.clear();

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

    if let Some(voice_paths_ctx) = req
        .get("voice_paths_contexto")
        .and_then(|v| v.as_object())
    {
        for (voice, path_val) in voice_paths_ctx {
            if let Some(path) = path_val.as_str() {
                let voice_lex = carregar_json_seguro(&Some(path.to_string()));
                if !voice_lex.is_empty() {
                    estado
                        .lexicon_voices_contexto
                        .insert(voice.clone(), Arc::new(voice_lex));
                }
            }
        }
    }

    rebuild_cache(estado);
    let t_end = Instant::now();

    let n_base = estado.lexicon_base.len();
    let n_base_ctx = estado.lexicon_base_contexto.len();
    let n_voices = estado.lexicon_voices.len();
    let n_voices_ctx = estado.lexicon_voices_contexto.len();
    let n_cached = estado.lexicon_cached.len();
    let n_cached_ctx = estado.lexicon_cached_contexto.len();

    eprintln!(
        "[lexicon] base={} | contexto={} | vozes={} (+{} contexto) | cache={}/{}",
        n_base, n_base_ctx, n_voices, n_voices_ctx, n_cached, n_cached_ctx
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

    let (lexico, lexico_contexto) = get_lexicons_for(estado, &voice, &overrides);
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
            lexico_contexto: Some(lexico_contexto.as_ref()),
            homografos: estado.homografos.as_ref(),
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

fn handle_process_batch(estado: &Estado, requisicao: &Value) -> RespostaBatch {
    let voz = requisicao
        .get("voice")
        .and_then(|valor| valor.as_str())
        .unwrap_or("")
        .to_string();

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

    let (lexico, lexico_contexto) = get_lexicons_for(estado, &voz, &overrides);

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

    let inicio_loop = Instant::now();

    for palavra in &palavras {
        let entrada = if precisa_normalizar(palavra) {
            normalizar(palavra, OpcoesNormalizar::default())
        } else {
            palavra.clone()
        };

        let opcoes = OpcoesFonemizar {
            normalizar: false,
            lexico: Some(lexico.as_ref()),
            lexico_contexto: Some(lexico_contexto.as_ref()),
            homografos: estado.homografos.as_ref(),
        };

        let fonema = fonemizar(&entrada, &opcoes);

        fonemas.insert(palavra.clone(), fonema);
    }

    let tempo_decorrido_ms = inicio_loop.elapsed().as_millis();

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

    // Léxico base (modo isolado).
    if let Some((caminho, lexico)) = carregar_arquivo("lexico_espeak.json") {
        eprintln!(
            "[lexicon] auto-carregado base: {} ({} entradas)",
            caminho.display(),
            lexico.len()
        );
        estado.lexicon_base = Arc::new(lexico);
    } else {
        eprintln!("[lexicon] nenhum lexico_espeak.json encontrado");
    }

    // Léxico contextual (modo sentença).
    if let Some((caminho, lexico)) = carregar_arquivo("lexico_espeak_contexto.json") {
        eprintln!(
            "[lexicon] auto-carregado contexto: {} ({} entradas)",
            caminho.display(),
            lexico.len()
        );
        estado.lexicon_base_contexto = Arc::new(lexico);
    } else {
        eprintln!("[lexicon] nenhum lexico_espeak_contexto.json encontrado");
    }

    // Homógrafos.
    if let Some(h) = carregar_homografos() {
        estado.homografos = Some(h);
        eprintln!("[homografos] carregado com sucesso");
    } else {
        eprintln!("[homografos] nenhum par regras+lexicon encontrado");
    }

    rebuild_cache(&mut estado);

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
                    "features": ["set_lexicon", "process", "process_batch", "homografos"],
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