#!/usr/bin/env python3
"""
compare_vozz.py

Compara vozz-js x vozz-rs x espeak-ng palavra a palavra, filtrando
apenas palavras que NÃO passam por normalização.

FILTRO: só compara palavras puramente alfabéticas que não são
abreviações conhecidas. Tudo que exige normalização (números, símbolos,
abreviações, datas, horas) é descartado da comparação — porque a
normalização é uma política, não uma questão de fonetização.

CORREÇÕES v2:

  1. Léxico agora é construído APENAS a partir da divergência do RS,
     não mais de `rs != espeak OR js != espeak`. Isso evita que a
     divergência do JS (que é ~50% do corpus e irrelevante para o
     léxico do RS) infle o arquivo.

  2. Comparação para inclusão no léxico é feita com `norm()` dos dois
     lados, alinhada com a comparação do relatório.

  3. Verificação e instalação automática do espeak-ng caso não esteja
     no PATH. Detecta apt/apt-get/dnf/yum/pacman/brew/apk/zypper.

Uso:
  python3 compare_vozz.py corpus.txt
  python3 compare_vozz.py corpus.txt --force-js
  python3 compare_vozz.py corpus.txt --force-espeak
  python3 compare_vozz.py corpus.txt --no-test
  python3 compare_vozz.py corpus.txt --incluir-normalizadas
  python3 compare_vozz.py corpus.txt --no-instalar-espeak
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import time
import unicodedata
from collections import Counter
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

# ---------------------------------------------------------------------------
# Configuração
# ---------------------------------------------------------------------------

PROJECT = Path(__file__).resolve().parent
WORKER = PROJECT / "target" / "release" / "phonemizer-worker"

CANDIDATOS_NODE_MODULES = [
    PROJECT / "node_modules",
    PROJECT / "compare_work" / "node_modules",
    PROJECT / "cache" / "node_modules",
]

ESPEAK_THREADS = 32

# Espelha ABREVIACOES do src/normalize.rs.
ABREVIACOES = {
    "sr", "sra", "srta", "dr", "dra", "prof", "profa", "eng",
    "av", "r", "pç", "ed", "apto", "ap", "pág", "pag", "fig",
    "obs", "ex", "etc", "cia", "ltda", "no", "tel", "cel",
    "kg", "km", "cm", "mm", "ml", "mg", "gb", "mb", "kb", "tb",
}


def log(m):
    print(m, flush=True)


def run(cmd, **kwargs):
    return subprocess.run(cmd, capture_output=True, text=True, **kwargs)


def norm(s):
    if not s:
        return ""
    s = s.replace("\u200d", "").replace("\u200c", "")
    s = unicodedata.normalize("NFD", s)
    s = s.replace("g", "ɡ")
    s = re.sub(r"ˈ{2,}", "ˈ", s)
    s = re.sub(r"\u0303{2,}", "\u0303", s)
    return s.strip()


def precisa_normalizar(palavra):
    if any(not c.isalpha() for c in palavra):
        return True
    return palavra.lower() in ABREVIACOES


def classificar_divergencia(a, b):
    if a.replace("ˌ", "") == b.replace("ˌ", ""):
        return "acento-secundario"
    if a.replace("ˈ", "") == b.replace("ˈ", ""):
        return "acento-primario"
    if a.replace("ˈ", "").replace("ˌ", "") == b.replace("ˈ", "").replace("ˌ", ""):
        return "posicao-acento"
    if len(a) == len(b):
        difs = sum(1 for x, y in zip(a, b) if x != y)
        if difs == 1:
            return "1-char-diff"
        if difs <= 3:
            return f"{difs}-char-diff"
        return "varios-chars-diff"
    dif_len = abs(len(a) - len(b))
    if dif_len == 1:
        return "1-char-len"
    if dif_len <= 3:
        return f"{dif_len}-char-len"
    return "estrutural"


def extrair_palavras(texto, incluir_normalizadas=False):
    tokens = re.findall(r"[\w'-]+", texto, re.UNICODE)
    s = set()
    descartadas = 0
    for t in tokens:
        limpo = t.lower().strip("'-")
        if len(limpo) < 2:
            continue
        partes = [limpo]
        if "-" in limpo:
            partes = [p.strip("'-") for p in limpo.split("-")]
            partes = [p for p in partes if len(p) >= 2]

        for p in partes:
            if not incluir_normalizadas and precisa_normalizar(p):
                descartadas += 1
                continue
            s.add(p)

    return sorted(s), descartadas


# ---------------------------------------------------------------------------
# Instalação do espeak-ng
# ---------------------------------------------------------------------------

def _comando_instalacao(base_cmd):
    """Prefixa com sudo se necessário e disponível."""
    if os.geteuid() == 0:
        return base_cmd
    if shutil.which("sudo"):
        return ["sudo"] + base_cmd
    return base_cmd


def verificar_espeak_instalado():
    """
    Verifica se espeak-ng está instalado. Se não estiver, tenta instalar
    automaticamente detectando o gerenciador de pacotes disponível.
    """
    if shutil.which("espeak-ng"):
        log("→ espeak-ng: ✓ encontrado")
        return True

    log("→ espeak-ng não encontrado. Tentando instalar automaticamente...")

    gerenciadores = [
        ("apt-get", ["apt-get", "install", "-y", "espeak-ng"]),
        ("apt",     ["apt",     "install", "-y", "espeak-ng"]),
        ("dnf",     ["dnf",     "install", "-y", "espeak-ng"]),
        ("yum",     ["yum",     "install", "-y", "espeak-ng"]),
        ("pacman",  ["pacman",  "-S", "--noconfirm", "espeak-ng"]),
        ("zypper",  ["zypper",  "install", "-y", "espeak-ng"]),
        ("apk",     ["apk",     "add", "espeak-ng"]),
        ("brew",    ["brew",    "install", "espeak-ng"]),
    ]

    for nome, base_cmd in gerenciadores:
        if not shutil.which(nome):
            continue

        log(f"  → gerenciador detectado: {nome}")
        cmd = _comando_instalacao(base_cmd)

        try:
            r = subprocess.run(
                cmd, capture_output=True, text=True, timeout=600
            )
            if r.returncode == 0:
                log(f"  ✓ instalado com {nome}")
                if shutil.which("espeak-ng"):
                    return True
                log("  ⚠ instalado, mas ainda não está no PATH.")
                log("    Reinicie o shell ou verifique manualmente.")
                return False
            else:
                msg = (r.stderr or r.stdout or "").strip().split("\n")[-1]
                log(f"  ⚠ falhou com {nome}: {msg[:200]}")
        except subprocess.TimeoutExpired:
            log(f"  ⚠ timeout com {nome}")
        except Exception as e:
            log(f"  ⚠ erro com {nome}: {e}")

    log("")
    log("  ✗ Não foi possível instalar automaticamente.")
    log("    Instale manualmente:")
    log("      Debian/Ubuntu: sudo apt-get install espeak-ng")
    log("      Fedora/RHEL:   sudo dnf install espeak-ng")
    log("      Arch:          sudo pacman -S espeak-ng")
    log("      Alpine:        sudo apk add espeak-ng")
    log("      openSUSE:      sudo zypper install espeak-ng")
    log("      macOS:         brew install espeak-ng")
    return False


# ---------------------------------------------------------------------------
# cargo test
# ---------------------------------------------------------------------------

def rodar_cargo_test():
    log("→ cargo test...")
    r = subprocess.run(
        ["cargo", "test", "--release", "--quiet"],
        cwd=str(PROJECT),
        capture_output=True,
        text=True,
        timeout=600,
    )

    saida = r.stdout + r.stderr
    for linha in saida.split("\n"):
        if "test result:" in linha:
            log(f"  {linha.strip()}")

    if r.returncode != 0:
        log("")
        log("  ⚠ CARGO TEST FALHOU. Detalhes:")
        for linha in saida.split("\n"):
            if "FAILED" in linha or "panicked" in linha or "assertion" in linha:
                log(f"  {linha}")
        log("")
        return False

    log("  ✓ Todos os testes passaram")
    return True


def garantir_worker():
    if WORKER.exists():
        return True
    log(f"→ Worker não encontrado em {WORKER}")
    log("→ Compilando com cargo build --release...")
    r = subprocess.run(
        ["cargo", "build", "--release", "--bin", "phonemizer-worker"],
        cwd=str(PROJECT),
        capture_output=True,
        text=True,
        timeout=600,
    )
    if r.returncode != 0:
        log("ERRO na compilação:")
        log(r.stderr[-2000:])
        return False
    if not WORKER.exists():
        log(f"ERRO: compilou mas {WORKER} não apareceu.")
        cargo_dir = os.environ.get("CARGO_TARGET_DIR", "(vazio)")
        log(f"  CARGO_TARGET_DIR={cargo_dir}")
        return False
    log("✓ Compilado")
    return True


# ---------------------------------------------------------------------------
# Vozz JS
# ---------------------------------------------------------------------------

def encontrar_node_modules():
    for nm in CANDIDATOS_NODE_MODULES:
        v = nm / "@pedrobef" / "vozz"
        if v.exists() and (v / "package.json").exists():
            return nm
    return None


def instalar_vozz_js(cache_dir):
    nm = encontrar_node_modules()
    if nm:
        return nm
    log("→ vozz-js não encontrado. Clonando em cache/node_modules...")
    target = cache_dir / "node_modules"
    vozz = target / "@pedrobef" / "vozz"
    vozz.parent.mkdir(parents=True, exist_ok=True)
    r = subprocess.run(
        ["git", "clone", "--depth", "1",
         "https://github.com/Pedro21062014/vozz.git", str(vozz)],
        capture_output=True, text=True,
    )
    if r.returncode != 0:
        log(f"ERRO: clone falhou\n{r.stderr[-400:]}")
        return None
    (cache_dir / "package.json").write_text(json.dumps({
        "name": "vozz-cache", "version": "1.0.0",
        "type": "module", "private": True,
    }, indent=2))
    return target


JS_RUNNER = r'''
import fs from 'fs';
import { pathToFileURL } from 'url';
const ep = process.argv[2];
const tp = process.argv[3];
const op = process.argv[4];
let fonemizar, lastErr;
try {
  const m = await import(pathToFileURL(ep).href);
  fonemizar = m.fonemizar;
  if (!fonemizar) { console.error('sem fonemizar'); process.exit(1); }
  console.error('JS carregado de:', ep);
} catch (e) { console.error('Falha:', e.message); process.exit(1); }
const words = JSON.parse(fs.readFileSync(tp, 'utf8'));
const out = {};
const t0 = process.hrtime.bigint();
for (const w of words) { try { out[w] = fonemizar(w); } catch (e) { out[w] = null; } }
const t1 = process.hrtime.bigint();
fs.writeFileSync(op, JSON.stringify(out));
console.error(`js-loop: ${words.length} em ${(Number(t1-t0)/1e6).toFixed(1)}ms`);
'''


def rodar_js(palavras, cache_dir, node_modules):
    log(f"→ Rodando vozz-g2p-js ({len(palavras)} palavras)...")
    vozz = node_modules / "@pedrobef" / "vozz"
    entrypoints = [
        vozz / "src" / "index.js",
        vozz / "src" / "g2p" / "index.js",
        vozz / "g2p" / "index.js",
        vozz / "index.js",
    ]
    entry = next((e for e in entrypoints if e.exists()), None)
    if entry is None:
        log("  ERRO: entrypoint não encontrado")
        return None, 0
    log(f"  entrypoint: {entry}")

    runner = cache_dir / "run_js.mjs"
    runner.write_text(JS_RUNNER, encoding="utf-8")
    tokens_js = cache_dir / "tokens_js.json"
    tokens_js.write_text(json.dumps(palavras, ensure_ascii=False), encoding="utf-8")

    out_path = cache_dir / "js.json"
    t0 = time.time()
    proc = subprocess.Popen(
        ["node", str(runner), str(entry), str(tokens_js), str(out_path)],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, cwd=str(cache_dir),
    )
    timing = 0.0
    for linha in proc.stderr:
        linha = linha.rstrip()
        if linha:
            log(f"  [js] {linha}")
            m = re.search(r"em ([\d.]+)ms", linha)
            if m:
                timing = float(m.group(1))
    proc.wait()
    wall = time.time() - t0
    if proc.returncode != 0:
        return None, 0
    log(f"→ JS: {wall:.2f}s (wall) | {timing:.1f}ms (loop)")
    return json.loads(out_path.read_text(encoding="utf-8")), timing


# ---------------------------------------------------------------------------
# Vozz RS
# ---------------------------------------------------------------------------

def rodar_rs(palavras, cache_dir):
    log(f"→ Rodando vozz-g2p-rs ({len(palavras)} palavras)...")
    req = {"action": "process_batch", "words": palavras, "voice": "", "overrides": {}}
    payload = json.dumps(req, ensure_ascii=False)
    log(f"  payload: {len(payload.encode('utf-8')) / 1024 / 1024:.1f} MB")

    t0 = time.time()
    r = subprocess.run(
        [str(WORKER)], input=payload + "\n",
        capture_output=True, text=True, timeout=1800,
    )
    wall = time.time() - t0
    if r.returncode != 0:
        log(f"ERRO: {r.stderr[-400:]}")
        return None, 0
    for l in r.stderr.strip().split("\n"):
        if l:
            log(f"  [rs] {l}")

    resp = json.loads(r.stdout.strip())
    fonemas = resp.get("phonemes", {})
    timing = float(resp.get("timing_ms", 0))

    out_path = cache_dir / "rs.json"
    out_path.write_text(json.dumps(fonemas, ensure_ascii=False), encoding="utf-8")
    log(f"→ RS: {wall:.2f}s (wall) | {timing:.1f}ms (loop)")
    return fonemas, timing


# ---------------------------------------------------------------------------
# espeak
# ---------------------------------------------------------------------------

def _espeak_uma(palavra):
    try:
        r = subprocess.run(
            ["espeak-ng", "-v", "pt-br", "--ipa=3", "-q", palavra],
            capture_output=True, text=True, timeout=10,
        )
        return r.stdout.strip() or None
    except Exception:
        return None


def rodar_espeak(palavras, cache_dir):
    log(f"→ Rodando espeak-ng ({len(palavras)} palavras, {ESPEAK_THREADS} threads)...")
    log(f"  (roda uma vez, fica salvo em cache/espeak.json)")
    t0 = time.time()
    resultado = {}

    with ThreadPoolExecutor(max_workers=ESPEAK_THREADS) as ex:
        futs = {ex.submit(_espeak_uma, w): w for w in palavras}
        concluidos = 0
        for f in as_completed(futs):
            w = futs[f]
            try:
                resultado[w] = f.result()
            except Exception:
                resultado[w] = None
            concluidos += 1
            if concluidos % 20000 == 0 or concluidos == len(palavras):
                dt = time.time() - t0
                log(f"  [espeak] {concluidos}/{len(palavras)} ({dt:.0f}s)")

    wall = time.time() - t0
    out_path = cache_dir / "espeak.json"
    out_path.write_text(json.dumps(resultado, ensure_ascii=False), encoding="utf-8")
    log(f"→ espeak: {wall:.2f}s")
    return resultado, wall


def espeak_cache_valido(cache_dir, palavras):
    p = cache_dir / "espeak.json"
    if not p.exists():
        return False
    try:
        data = json.loads(p.read_text(encoding="utf-8"))
        if not isinstance(data, dict):
            return False
        return len(data) >= len(palavras) * 0.9
    except Exception:
        return False


# ---------------------------------------------------------------------------
# Comparação
# ---------------------------------------------------------------------------

def comparar(a, b, na, nb):
    match = 0
    total = 0
    pulados = 0
    difs = []
    for p, va in a.items():
        vb = b.get(p)
        if va is None or vb is None:
            pulados += 1
            continue
        total += 1
        if norm(va) == norm(vb):
            match += 1
        else:
            difs.append((p, norm(va), norm(vb)))
    return {
        "name": f"{na} vs {nb}",
        "total": total,
        "match": match,
        "pulados": pulados,
        "difs": difs,
    }


def agrupar_por_categoria(difs):
    cats = Counter()
    exs = {}
    for palavra, a, b in difs:
        cat = classificar_divergencia(a, b)
        cats[cat] += 1
        if cat not in exs:
            exs[cat] = []
        if len(exs[cat]) < 3:
            exs[cat].append((palavra, a, b))
    return cats, exs


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Compara vozz-js x vozz-rs x espeak-ng (só palavras sem normalização)."
    )
    parser.add_argument("corpus", help="Caminho do corpus.txt")
    parser.add_argument("--cache-dir", default=None,
                        help="Diretório de cache (default: <projeto>/cache)")
    parser.add_argument("--force-js", action="store_true",
                        help="Regerar js.json mesmo se existir")
    parser.add_argument("--force-espeak", action="store_true",
                        help="Regerar espeak.json mesmo se existir")
    parser.add_argument("--force-all", action="store_true",
                        help="Regerar tudo")
    parser.add_argument("--no-test", action="store_true",
                        help="Pular cargo test")
    parser.add_argument("--incluir-normalizadas", action="store_true",
                        help="Não filtrar palavras que precisam de normalização")
    parser.add_argument("--no-instalar-espeak", action="store_true",
                        help="Não tentar instalar espeak-ng automaticamente")
    args = parser.parse_args()

    corpus_path = Path(args.corpus).resolve()
    if not corpus_path.is_file():
        log(f"ERRO: corpus não encontrado: {corpus_path}")
        sys.exit(1)

    cache_dir = Path(args.cache_dir).resolve() if args.cache_dir else PROJECT / "cache"
    cache_dir.mkdir(parents=True, exist_ok=True)

    log("=" * 60)
    log("COMPARAÇÃO vozz-js x vozz-rs x espeak-ng")
    log("=" * 60)
    log(f"Projeto:  {PROJECT}")
    log(f"Cache:    {cache_dir}")
    log(f"Corpus:   {corpus_path}")
    if args.incluir_normalizadas:
        log(f"Filtro:   DESLIGADO (inclui normalizadas)")
    else:
        log(f"Filtro:   SÓ PALAVRAS SEM NORMALIZAÇÃO")
    log("")

    # 1. cargo test
    if not args.no_test:
        if not rodar_cargo_test():
            log("Abortando: corrija os testes antes de continuar.")
            sys.exit(1)
        log("")

    # 2. Worker
    if not garantir_worker():
        sys.exit(1)

    # 3. Lê corpus + extrai palavras
    texto = corpus_path.read_text(encoding="utf-8")
    corpus_hash = hashlib.sha256(texto.encode("utf-8")).hexdigest()[:16]
    log(f"→ Hash do corpus: {corpus_hash}")

    hash_path = cache_dir / "corpus.hash"
    hash_ant = hash_path.read_text().strip() if hash_path.exists() else None
    corpus_mudou = hash_ant != corpus_hash
    if hash_ant is None:
        log("→ Primeira execução.")
    elif corpus_mudou:
        log(f"→ ⚠ Corpus MUDOU (antes: {hash_ant})")
    else:
        log("→ Corpus não mudou.")

    palavras, descartadas = extrair_palavras(
        texto, incluir_normalizadas=args.incluir_normalizadas
    )
    log(f"→ {len(palavras)} palavras únicas (fonetização pura)")
    if descartadas:
        log(f"   {descartadas} tokens descartados (precisam normalização)")
    (cache_dir / "tokens.json").write_text(
        json.dumps(palavras, ensure_ascii=False), encoding="utf-8"
    )

    # 4. Decide o que regerar
    js_path = cache_dir / "js.json"
    espeak_path = cache_dir / "espeak.json"

    refazer_js = args.force_js or args.force_all or not js_path.exists()
    refazer_espeak = args.force_espeak or args.force_all

    if not refazer_espeak:
        if not espeak_cache_valido(cache_dir, palavras):
            if espeak_path.exists():
                log("→ espeak.json incompleto. Regerando.")
            else:
                log("→ espeak.json não existe. Gerando (leva ~5-10 min).")
            refazer_espeak = True
        else:
            log("→ espeak.json válido. Reaproveitando cache.")

    # 5. Verifica espeak-ng ANTES de decidir se vai rodar
    if refazer_espeak or not args.no_test:
        if not args.no_instalar_espeak:
            if not verificar_espeak_instalado():
                log("")
                log("Abortando: espeak-ng é necessário.")
                sys.exit(1)

    # 6. js
    t_js_loop = 0.0
    if refazer_js:
        nm = instalar_vozz_js(cache_dir)
        if nm is None:
            log("ERRO: vozz-js não disponível.")
            sys.exit(1)
        log(f"→ node_modules: {nm}")
        js, t_js_loop = rodar_js(palavras, cache_dir, nm)
        if js is None:
            log("ERRO: js falhou.")
            sys.exit(1)
    else:
        log("→ js.json do cache.")
        js = json.loads(js_path.read_text(encoding="utf-8"))

    # 7. espeak
    t_espeak_wall = 0.0
    if refazer_espeak:
        espeak, t_espeak_wall = rodar_espeak(palavras, cache_dir)
        if espeak is None:
            espeak = {}
    else:
        log("→ espeak.json do cache.")
        espeak = json.loads(espeak_path.read_text(encoding="utf-8"))

    # 8. rs
    rs, t_rs_loop = rodar_rs(palavras, cache_dir)
    if rs is None:
        log("ERRO: rs falhou.")
        sys.exit(1)

    # 9. Tempos
    (cache_dir / "tempos.json").write_text(json.dumps({
        "js_loop_ms": t_js_loop,
        "rs_loop_ms": t_rs_loop,
        "espeak_wall_s": t_espeak_wall,
        "corpus_hash": corpus_hash,
        "n_palavras": len(palavras),
        "n_descartadas": descartadas,
    }, indent=2), encoding="utf-8")
    hash_path.write_text(corpus_hash, encoding="utf-8")

    # 10. Comparações
    log("")
    log("=" * 72)
    log("SUMÁRIO")
    log("=" * 72)

    pares = [comparar(js, rs, "vozz-js", "vozz-rs")]
    if espeak:
        pares.append(comparar(rs, espeak, "vozz-rs", "espeak"))
        pares.append(comparar(js, espeak, "vozz-js", "espeak"))

    for p in pares:
        if p["total"]:
            pct = p["match"] / p["total"] * 100
            log(
                f"  {p['name']:<20} {p['match']:>7}/{p['total']:<7} = {pct:.2f}%"
                f"  (sem saída: {p['pulados']})"
            )

    # 11. Léxico — CORRIGIDO: só divergência do RS, comparação normalizada.
    n_lex = 0
    if espeak:
        lexico = {}
        for p in palavras:
            e = espeak.get(p)
            if not e:
                continue
            rs_norm = norm(rs.get(p) or "")
            e_norm = norm(e)
            if rs_norm != e_norm:
                lexico[p] = e_norm
        lex_path = cache_dir / "lexico_espeak.json"
        lex_path.write_text(
            json.dumps(lexico, ensure_ascii=False, indent=2, sort_keys=True),
            encoding="utf-8",
        )
        n_lex = len(lexico)
        log(f"\nLéxico: {n_lex} entradas → {lex_path}")
        log(f"  ({n_lex / len(palavras) * 100:.1f}% do corpus)")

    # 12. Relatório
    linhas = []
    linhas.append("=" * 100)
    linhas.append("RELATÓRIO — vozz-js x vozz-rs x espeak-ng pt-br")
    linhas.append("=" * 100)
    linhas.append(f"Corpus: {corpus_path} (hash={corpus_hash})")
    linhas.append(f"Palavras únicas (fonetização pura): {len(palavras)}")
    if descartadas:
        linhas.append(f"Descartadas (precisam normalização): {descartadas}")
    linhas.append("")
    linhas.append("TEMPOS")
    if t_js_loop:
        linhas.append(f"  js loop     : {t_js_loop:>10.1f} ms")
    linhas.append(f"  rs loop     : {t_rs_loop:>10.1f} ms")
    if t_espeak_wall:
        linhas.append(f"  espeak wall : {t_espeak_wall:>10.1f} s")
    linhas.append(f"  Léxico      : {n_lex} entradas ({n_lex / len(palavras) * 100:.1f}%)")
    linhas.append("")
    linhas.append("SUMÁRIO")
    for p in pares:
        if p["total"]:
            pct = p["match"] / p["total"] * 100
            linhas.append(
                f"  {p['name']:<20} {p['match']:>7}/{p['total']:<7} = {pct:.2f}%"
                f"  (sem saída: {p['pulados']})"
            )
        else:
            linhas.append(f"  {p['name']:<20} (sem dados)")

    for p in pares:
        if not p["difs"]:
            continue
        linhas.append("")
        linhas.append("=" * 100)
        linhas.append(f"{p['name']} — {len(p['difs'])} divergências")
        linhas.append("=" * 100)

        cats, exs = agrupar_por_categoria(p["difs"])
        total_difs = len(p["difs"])

        linhas.append("")
        linhas.append("CATEGORIAS")
        for cat, cnt in cats.most_common():
            pct = cnt / total_difs * 100
            linhas.append(f"  {cat:<25} {cnt:>7}  ({pct:>5.1f}%)")

        linhas.append("")
        linhas.append("EXEMPLOS POR CATEGORIA (até 3 cada)")
        for cat, cnt in cats.most_common(10):
            linhas.append("")
            linhas.append(f"  [{cat}] {cnt} ocorrências")
            for palavra, a, b in exs.get(cat, []):
                a_d = a if len(a) <= 35 else a[:32] + "..."
                b_d = b if len(b) <= 35 else b[:32] + "..."
                linhas.append(f"    {palavra:<18} {a_d:<38} → {b_d}")

    rel_path = cache_dir / "relatorio.txt"
    rel_path.write_text("\n".join(linhas), encoding="utf-8")
    log(f"\nRelatório: {rel_path}")

    log("")
    log("=" * 72)
    log("PREVIEW")
    log("=" * 72)
    for l in linhas[:80]:
        log(l)


if __name__ == "__main__":
    main()