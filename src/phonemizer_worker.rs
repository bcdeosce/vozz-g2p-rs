//! Worker persistente: JSON via stdin/stdout.
//!
//! Ações:
//!   version
//!   set_lexicon
//!   process                 → IPA puro por sentença
//!   process_piper           → tokens Piper por sentença
//!   process_piper_chunks    → chunks + pausas prontos para síntese
//!   process_batch

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use vozz_g2p_rs::g2p::{fonemizar, OpcoesFonemizar};
use vozz_g2p_rs::lexicon_homografos::LexiconHomografos;
use vozz_g2p_rs::normalize::{normalizar, precisa_normalizar, OpcoesNormalizar};
use vozz_g2p_rs::piper::ipa_para_piper;
use vozz_g2p_rs::piper_pipeline::{self, preparar_chunks};
use vozz_g2p_rs::splitter::dividir_em_sentencas;
use vozz_g2p_rs::tagger::Tagger;


const SLOW_THRESHOLD_MS: u128 = 50;
const BUILD_ID: &str = "2024-11-piper-chunks-v2";

// ... (mantém Estado, RespostaOk, RespostaErro, RespostaProcesso,
//      Sentenca, RespostaProcessoPiper, SentencaPiper, RespostaBatch
//      exatamente como no arquivo atual) ...

struct Estado {
    lexicon_base: Arc<HashMap<String, String>>,
    lexicon_base_contexto: Arc<HashMap<String, String>>,
    lexicon_voices: HashMap<String, Arc<HashMap<String, String>>>,
    lexicon_voices_contexto: HashMap<String, Arc<HashMap<String, String>>>,
    lexicon_cached: HashMap<String, Arc<HashMap<String, String>>>,
    lexicon_cached_contexto: HashMap<String, Arc<HashMap<String, String>>>,
    homografos: Option<LexiconHomografos>,
    tagger: Option<Tagger>,
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
            tagger: None,
        }
    }
}

// ─── Respostas ────────────────────────────────────────────────────────

#[derive(Serialize)] struct RespostaOk { ok: bool }
#[derive(Serialize)] struct RespostaErro { error: String }
#[derive(Serialize)] struct RespostaProcesso { sentences: Vec<Sentenca> }
#[derive(Serialize)] struct Sentenca { text: String, phonemes: String }
#[derive(Serialize)] struct RespostaProcessoPiper { sentences: Vec<SentencaPiper> }
#[derive(Serialize)] struct SentencaPiper { text: String, phonemes: Vec<String> }
#[derive(Serialize)] struct RespostaBatch {
    phonemes: HashMap<String, String>, timing_ms: u128, total: usize,
}

// ─── Carregamento de léxicos (idêntico ao atual) ──────────────────────

fn carregar_json_seguro(path: &Option<String>) -> HashMap<String, String> {
    let Some(p) = path.as_deref() else { return HashMap::new(); };
    match std::fs::read_to_string(p) {
        Ok(c) => match serde_json::from_str::<HashMap<String, String>>(&c) {
            Ok(m) => m,
            Err(e) => { eprintln!("[lexicon] parse {}: {}", p, e); HashMap::new() }
        },
        Err(e) => { eprintln!("[lexicon] ler {}: {}", p, e); HashMap::new() }
    }
}

fn rebuild_cache(e: &mut Estado) {
    e.lexicon_cached.clear();
    e.lexicon_cached_contexto.clear();
    for (v, vl) in &e.lexicon_voices {
        let mut m = (*e.lexicon_base).clone();
        for (k, val) in vl.iter() { m.insert(k.clone(), val.clone()); }
        e.lexicon_cached.insert(v.clone(), Arc::new(m));
    }
    for (v, vl) in &e.lexicon_voices_contexto {
        let mut m = (*e.lexicon_base_contexto).clone();
        for (k, val) in vl.iter() { m.insert(k.clone(), val.clone()); }
        e.lexicon_cached_contexto.insert(v.clone(), Arc::new(m));
    }
    e.lexicon_cached.insert("__default__".to_string(), Arc::clone(&e.lexicon_base));
    e.lexicon_cached_contexto.insert("__default__".to_string(),
                                     Arc::clone(&e.lexicon_base_contexto));
}

fn get_lexicons_for(e: &Estado, voice: &str, overrides: &HashMap<String, String>)
    -> (Arc<HashMap<String, String>>, Arc<HashMap<String, String>>)
{
    let ci = e.lexicon_cached.get(voice)
        .or_else(|| e.lexicon_cached.get("__default__")).cloned()
        .unwrap_or_else(|| Arc::clone(&e.lexicon_base));
    let cc = e.lexicon_cached_contexto.get(voice)
        .or_else(|| e.lexicon_cached_contexto.get("__default__")).cloned()
        .unwrap_or_else(|| Arc::clone(&e.lexicon_base_contexto));
    if overrides.is_empty() { return (ci, cc); }
    let mut mi = (*ci).clone(); let mut mc = (*cc).clone();
    for (k, v) in overrides { mi.insert(k.clone(), v.clone()); mc.insert(k.clone(), v.clone()); }
    (Arc::new(mi), Arc::new(mc))
}

fn caminhos_arquivo(nome: &str) -> Vec<PathBuf> {
    let mut c = Vec::new();
    let stem = nome.trim_end_matches(".json").to_uppercase();
    if let Ok(p) = std::env::var(&format!("VOZZ_{}", stem)) {
        if !p.is_empty() { c.push(PathBuf::from(p)); }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() { c.push(d.join(nome)); }
    }
    c.push(PathBuf::from(nome));
    c.push(PathBuf::from("data").join(nome));
    c.push(PathBuf::from("cache").join(nome));
    c
}

fn carregar_arquivo(nome: &str) -> Option<(PathBuf, HashMap<String, String>)> {
    for c in caminhos_arquivo(nome) {
        if !c.is_file() { continue; }
        if let Ok(s) = std::fs::read_to_string(&c) {
            if let Ok(m) = serde_json::from_str::<HashMap<String, String>>(&s) {
                if !m.is_empty() { return Some((c, m)); }
            }
        }
    }
    None
}

fn carregar_homografos_lexico() -> Option<(LexiconHomografos, PathBuf)> {
    let path = caminhos_arquivo("lexicon_homografos.json").into_iter()
        .find(|p| p.is_file())?;
    LexiconHomografos::from_path(&path).ok().map(|l| (l, path))
}

fn caminhos_tagger_data() -> Vec<String> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("BCDE_TAGGER_DATA") {
        if !p.is_empty() { v.push(p); }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            v.push(d.join("../BCDE-tagger/data").to_string_lossy().to_string());
            v.push(d.join("data").to_string_lossy().to_string());
        }
    }
    v.push("/content/BCDE-tagger/data".to_string());
    v.push("data".to_string());
    v
}

fn carregar_tagger() -> Option<Tagger> {
    for data in caminhos_tagger_data() {
        if !std::path::Path::new(&data).is_dir() { continue; }
        match vozz_g2p_rs::tagger::carregar_tagger(&data) {
            Ok(t) => { eprintln!("[tagger] carregado de {}", data); return Some(t); }
            Err(e) => eprintln!("[tagger] falha em {}: {}", data, e),
        }
    }
    None
}

// ─── Handlers ─────────────────────────────────────────────────────────

fn handle_set_lexicon(e: &mut Estado, req: &Value) {
    let t0 = Instant::now();
    let base = carregar_json_seguro(&req.get("base_path").and_then(|v| v.as_str()).map(String::from));
    let global = carregar_json_seguro(&req.get("global_path").and_then(|v| v.as_str()).map(String::from));
    let ctx = carregar_json_seguro(&req.get("context_path").and_then(|v| v.as_str()).map(String::from));
    let mut lb = base;
    for (k, v) in global { lb.insert(k, v); }
    e.lexicon_base = Arc::new(lb);
    e.lexicon_base_contexto = Arc::new(ctx);
    e.lexicon_voices.clear();
    e.lexicon_voices_contexto.clear();

    if let Some(vp) = req.get("voice_paths").and_then(|v| v.as_object()) {
        for (v, pv) in vp {
            if let Some(p) = pv.as_str() {
                let vl = carregar_json_seguro(&Some(p.to_string()));
                if !vl.is_empty() { e.lexicon_voices.insert(v.clone(), Arc::new(vl)); }
            }
        }
    }
    if let Some(vp) = req.get("voice_paths_contexto").and_then(|v| v.as_object()) {
        for (v, pv) in vp {
            if let Some(p) = pv.as_str() {
                let vl = carregar_json_seguro(&Some(p.to_string()));
                if !vl.is_empty() { e.lexicon_voices_contexto.insert(v.clone(), Arc::new(vl)); }
            }
        }
    }
    rebuild_cache(e);
    eprintln!("[lexicon] base={} ctx={} vozes={} (+{} ctx) tempo={}ms",
        e.lexicon_base.len(), e.lexicon_base_contexto.len(),
        e.lexicon_voices.len(), e.lexicon_voices_contexto.len(),
        t0.elapsed().as_millis());
}

fn extrair_overrides(req: &Value) -> HashMap<String, String> {
    req.get("overrides").and_then(|v| v.as_object())
        .map(|o| o.iter().filter_map(|(k, v)| v.as_str()
            .map(|s| (k.clone(), s.to_string()))).collect())
        .unwrap_or_default()
}

fn opcoes_para<'a>(e: &'a Estado, l: &'a HashMap<String, String>,
                    lc: &'a HashMap<String, String>) -> OpcoesFonemizar<'a> {
    OpcoesFonemizar {
        normalizar: false,
        lexico: Some(l),
        lexico_contexto: Some(lc),
        homografos: e.homografos.as_ref(),
        tagger: e.tagger.as_ref(),
    }
}

fn handle_process(e: &Estado, req: &Value) -> RespostaProcesso {
    let text = req.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let voice = req.get("voice").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let overrides = extrair_overrides(req);
    let (l, lc) = get_lexicons_for(e, &voice, &overrides);
    let norm = normalizar(&text, OpcoesNormalizar::default());
    let sents = dividir_em_sentencas(&norm);
    let mut out = Vec::with_capacity(sents.len());
    for s in &sents {
        let o = opcoes_para(e, &l, &lc);
        out.push(Sentenca { text: s.to_string(), phonemes: fonemizar(s, &o) });
    }
    RespostaProcesso { sentences: out }
}

fn handle_process_piper(e: &Estado, req: &Value) -> RespostaProcessoPiper {
    let text = req.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let voice = req.get("voice").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let overrides = extrair_overrides(req);
    let (l, lc) = get_lexicons_for(e, &voice, &overrides);
    let norm = normalizar(&text, OpcoesNormalizar::default());
    let sents = dividir_em_sentencas(&norm);
    let mut out = Vec::with_capacity(sents.len());
    for s in &sents {
        let o = opcoes_para(e, &l, &lc);
        let ipa = fonemizar(s, &o);
        out.push(SentencaPiper { text: s.to_string(), phonemes: ipa_para_piper(&ipa) });
    }
    RespostaProcessoPiper { sentences: out }
}

/// NOVO: devolve chunks com pausas prontos para síntese.
///
/// Request:
///   {"action":"process_piper_chunks","text":"...","voice":"","emocao":"neutro",
///    "overrides":{}}
///
/// Response:
///   {
///     "ipa_completo": "...",
///     "emocao": "neutro",
///     "chunks": [ { "fragments":[{"ipa":"...","pausa_ms":180}, ...],
///                   "length_scale":1.0,
///                   "pausa_apos_ms":500 }, ... ]
///   }
fn handle_process_piper_chunks(e: &Estado, req: &Value) -> Value {
    let text = req.get("text").and_then(|v| v.as_str()).unwrap_or("");
    let voice = req.get("voice").and_then(|v| v.as_str()).unwrap_or("");
    let overrides = extrair_overrides(req);

    let (l, lc) = get_lexicons_for(e, voice, &overrides);
    let norm = normalizar(text, OpcoesNormalizar::default());
    let sents = dividir_em_sentencas(&norm);

    let mut ipa_piper = String::new();
    for s in &sents {
        let o = opcoes_para(e, &l, &lc);
        let ipa = fonemizar(s, &o);
        let tokens = ipa_para_piper(&ipa);
        let pedaco: String = tokens.iter().map(|s| s.as_str()).collect();
        if !ipa_piper.is_empty() { ipa_piper.push(' '); }
        ipa_piper.push_str(pedaco.trim());
    }

    let chunks = piper_pipeline::preparar_chunks(&ipa_piper);

    serde_json::json!({
        "ipa_piper": ipa_piper,
        "chunks": chunks,
    })
}

fn handle_process_batch(e: &Estado, req: &Value) -> RespostaBatch {
    let voz = req.get("voice").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let overrides = extrair_overrides(req);
    let (l, lc) = get_lexicons_for(e, &voz, &overrides);
    let palavras: Vec<String> = req.get("words").and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let total = palavras.len();
    let mut fonemas = HashMap::with_capacity(total);
    let t0 = Instant::now();
    for p in &palavras {
        let entrada = if precisa_normalizar(p) {
            normalizar(p, OpcoesNormalizar::default())
        } else { p.clone() };
        let o = opcoes_para(e, &l, &lc);
        fonemas.insert(p.clone(), fonemizar(&entrada, &o));
    }
    let dt = t0.elapsed().as_millis();
    RespostaBatch { phonemes: fonemas, timing_ms: dt, total }
}

// ─── Main ─────────────────────────────────────────────────────────────

fn main() {
    let mut estado = Estado::novo();

    if let Some((c, l)) = carregar_arquivo("lexico_espeak.json") {
        eprintln!("[lexicon] base: {} ({} entradas)", c.display(), l.len());
        estado.lexicon_base = Arc::new(l);
    }
    if let Some((c, l)) = carregar_arquivo("lexico_espeak_contexto.json") {
        eprintln!("[lexicon] ctx: {} ({} entradas)", c.display(), l.len());
        estado.lexicon_base_contexto = Arc::new(l);
    }
    if let Some((h, p)) = carregar_homografos_lexico() {
        eprintln!("[homografos] {} ({} palavras)", p.display(), h.len());
        estado.homografos = Some(h);
    }
    match carregar_tagger() {
        Some(t) => estado.tagger = Some(t),
        None => eprintln!("[tagger] BCDE NÃO carregado"),
    }
    rebuild_cache(&mut estado);

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line_res in stdin.lock().lines() {
        let line = match line_res { Ok(l) => l, Err(_) => break };
        if line.trim().is_empty() { continue; }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let r = serde_json::json!({"error": format!("JSON parse: {} (build {})", e, BUILD_ID)});
                let _ = writeln!(out, "{}", r); let _ = out.flush(); continue;
            }
        };

        let action = req.get("action").and_then(|v| v.as_str()).unwrap_or("");
        let resposta: String = match action {
            "version" => serde_json::to_string(&serde_json::json!({
                "build": BUILD_ID,
                "features": ["set_lexicon", "process", "process_piper",
                             "process_piper_chunks", "process_batch",
                             "tagger", "trema"],
            })).unwrap_or_else(|_| "{}".to_string()),

            "set_lexicon" => {
                handle_set_lexicon(&mut estado, &req);
                serde_json::to_string(&RespostaOk { ok: true }).unwrap_or_else(|_| "{\"ok\":true}".to_string())
            }
            "process" => serde_json::to_string(&handle_process(&estado, &req))
                .unwrap_or_else(|_| "{\"error\":\"serialization\"}".to_string()),
            "process_piper" => serde_json::to_string(&handle_process_piper(&estado, &req))
                .unwrap_or_else(|_| "{\"error\":\"serialization\"}".to_string()),
            "process_piper_chunks" => {
                let r = handle_process_piper_chunks(&estado, &req);
                serde_json::to_string(&r).unwrap_or_else(|_| "{\"error\":\"serialization\"}".to_string())
            }
            "process_batch" => serde_json::to_string(&handle_process_batch(&estado, &req))
                .unwrap_or_else(|_| "{\"error\":\"serialization\"}".to_string()),
            other => serde_json::to_string(&RespostaErro {
                error: format!("Unknown action: {} (build {})", other, BUILD_ID),
            }).unwrap_or_else(|_| "{\"error\":\"serialization\"}".to_string()),
        };
        let _ = writeln!(out, "{}", resposta);
        let _ = out.flush();
    }
}
