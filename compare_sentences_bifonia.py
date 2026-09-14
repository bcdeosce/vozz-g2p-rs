#!/usr/bin/env python3
"""
compare_sentences_bifonia.py

Avalia o pipeline completo (vozz-js x vozz-rs x espeak-ng) sobre o corpus
do Bifonia (`sentences_bifonia.txt`), com foco em:

  1. Cache dos resultados do espeak em `espeak_sentences_bifonia.json`,
     para que reruns subsequentes não paguem o custo de 2 minutos do espeak.

  2. Avaliação específica de homógrafos: para cada palavra presente em
     `homograph_rules_v2.json`, mede se o pipeline acertou o IPA.

  3. Distribuição de erros por sentença: quantas frases têm 1, 2, 3, 4
     ou 5+ erros, com percentuais.

Uso:
  python3 compare_sentences_bifonia.py sentences_bifonia.txt
  python3 compare_sentences_bifonia.py sentences_bifonia.txt --limite 1000
  python3 compare_sentences_bifonia.py sentences_bifonia.txt --force-espeak
"""

import argparse
import json
import re
import subprocess
import sys
import time
import unicodedata
from collections import Counter, defaultdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path


# ---------------------------------------------------------------------------
# Configuração
# ---------------------------------------------------------------------------

DIRETORIO_PROJETO = Path(__file__).resolve().parent
CAMINHO_WORKER = DIRETORIO_PROJETO / "target" / "release" / "phonemizer-worker"
CAMINHO_CACHE = DIRETORIO_PROJETO / "cache"
CAMINHO_ESPEAK_CACHE = CAMINHO_CACHE / "espeak_sentences_bifonia.json"
CAMINHO_HOMOGRAFOS = CAMINHO_CACHE / "homograph_rules_v2.json"

CAMINHOS_NODE_MODULES = [
    DIRETORIO_PROJETO / "node_modules",
    DIRETORIO_PROJETO / "compare_work" / "node_modules",
    CAMINHO_CACHE / "node_modules",
]

QUANTIDADE_THREADS_ESPEAK = 32

ABREVIACOES = {
    "sr", "sra", "srta", "dr", "dra", "prof", "profa", "eng",
    "av", "r", "pç", "ed", "apto", "ap", "pág", "pag", "fig",
    "obs", "ex", "etc", "cia", "ltda", "no", "tel", "cel",
    "kg", "km", "cm", "mm", "ml", "mg", "gb", "mb", "kb", "tb",
}

VOGAIS_PORTUGUES = "aeiouáéíóúâêôãõàü"

RE_TOKEN_TEXTO = re.compile(r"[\w'-]+", re.UNICODE)
RE_PONTUACAO_BORDA = re.compile(
    r"^[;:,.!?¡¿—…\"«»“”(){}]+|[;:,.!?¡¿—…\"«»“”(){}]+$"
)


# ---------------------------------------------------------------------------
# Utilidades
# ---------------------------------------------------------------------------

def imprimir(msg):
    print(msg, flush=True)


def normalizar_ipa(ipa):
    if not ipa:
        return ""
    ipa = ipa.replace("\u200d", "").replace("\u200c", "")
    ipa = unicodedata.normalize("NFD", ipa)
    ipa = ipa.replace("g", "ɡ")
    ipa = re.sub(r"ˈ{2,}", "ˈ", ipa)
    ipa = re.sub(r"\u0303{2,}", "\u0303", ipa)
    return ipa.strip()


def tokenizar_texto(texto):
    return RE_TOKEN_TEXTO.findall(texto)


def tokenizar_ipa(ipa):
    if not ipa:
        return []
    return [t for t in (RE_PONTUACAO_BORDA.sub("", x) for x in ipa.split()) if t]


def eh_puramente_numerico(p):
    return bool(p) and all(c.isdigit() and c.isascii() for c in p)


def eh_sigla_sem_vogal(p):
    if not (2 <= len(p) <= 6) or not p.isalpha():
        return False
    return not any(c in VOGAIS_PORTUGUES for c in p)


def precisa_normalizar(p):
    if any(not c.isalpha() for c in p):
        return True
    if p.lower() in ABREVIACOES:
        return True
    return eh_sigla_sem_vogal(p)


def carregar_excecoes():
    caminho = CAMINHO_CACHE / "exceptions.json"
    if not caminho.exists():
        return set()
    try:
        return set(json.loads(caminho.read_text(encoding="utf-8")))
    except Exception:
        return set()


def filtrar_sentencas(sentencas, excecoes):
    filtradas = []
    descartadas = 0
    for s in sentencas:
        pular = False
        for t in RE_TOKEN_TEXTO.findall(s):
            limpo = t.lower().strip("'-")
            if len(limpo) < 2:
                continue
            if eh_puramente_numerico(limpo):
                pular = True
                break
            if precisa_normalizar(limpo):
                pular = True
                break
            if limpo in excecoes:
                pular = True
                break
        if pular:
            descartadas += 1
        else:
            filtradas.append(s)
    return filtradas, descartadas


# ---------------------------------------------------------------------------
# Motores
# ---------------------------------------------------------------------------

def encontrar_node_modules():
    for d in CAMINHOS_NODE_MODULES:
        vozz = d / "@pedrobef" / "vozz"
        if vozz.exists() and (vozz / "package.json").exists():
            return d
    return None


SCRIPT_RUNNER_JS = r'''
import fs from 'fs';
import { pathToFileURL } from 'url';
const entrypoint = process.argv[2];
const caminhoSentencas = process.argv[3];
const caminhoSaida = process.argv[4];
let fonemizar;
try {
  const modulo = await import(pathToFileURL(entrypoint).href);
  fonemizar = modulo.fonemizar;
  if (!fonemizar) { console.error('sem fonemizar'); process.exit(1); }
  console.error('JS carregado de:', entrypoint);
} catch (erro) { console.error('Falha:', erro.message); process.exit(1); }
const sentencas = JSON.parse(fs.readFileSync(caminhoSentencas, 'utf8'));
const saida = [];
const inicio = process.hrtime.bigint();
for (const s of sentencas) {
  try { saida.push(fonemizar(s)); } catch (e) { saida.push(null); }
}
const fim = process.hrtime.bigint();
fs.writeFileSync(caminhoSaida, JSON.stringify(saida));
console.error(`js-loop: ${sentencas.length} em ${(Number(fim-inicio)/1e6).toFixed(1)}ms`);
'''


def rodar_vozz_js(sentencas, diretorio_node_modules, diretorio_cache):
    imprimir(f"→ Rodando vozz-js em {len(sentencas)} sentenças...")
    vozz = diretorio_node_modules / "@pedrobef" / "vozz"
    candidatos = [
        vozz / "src" / "index.js",
        vozz / "src" / "g2p" / "index.js",
        vozz / "g2p" / "index.js",
        vozz / "index.js",
    ]
    entrypoint = next((e for e in candidatos if e.exists()), None)
    if entrypoint is None:
        imprimir("  ERRO: entrypoint do JS não encontrado")
        return None

    runner = diretorio_cache / "run_js_bifonia.mjs"
    runner.write_text(SCRIPT_RUNNER_JS, encoding="utf-8")
    caminho_sent = diretorio_cache / "bifonia_sentencas_js.json"
    caminho_sent.write_text(
        json.dumps(sentencas, ensure_ascii=False), encoding="utf-8"
    )
    caminho_saida = diretorio_cache / "bifonia_js_saida.json"

    inicio = time.time()
    proc = subprocess.Popen(
        ["node", str(runner), str(entrypoint),
         str(caminho_sent), str(caminho_saida)],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, cwd=str(diretorio_cache),
    )
    for linha in proc.stderr:
        linha = linha.rstrip()
        if linha:
            imprimir(f"  [js] {linha}")
    proc.wait()
    tempo = time.time() - inicio
    if proc.returncode != 0:
        imprimir(f"ERRO: js falhou (rc={proc.returncode})")
        return None
    imprimir(f"→ JS: {tempo:.2f}s")
    return json.loads(caminho_saida.read_text(encoding="utf-8"))


def rodar_vozz_rs(sentencas, caminho_worker):
    imprimir(f"→ Rodando vozz-rs em {len(sentencas)} sentenças...")
    linhas = []
    for s in sentencas:
        req = {"action": "process", "text": s, "voice": "", "overrides": {}}
        linhas.append(json.dumps(req, ensure_ascii=False))
    entrada = "\n".join(linhas) + "\n"

    inicio = time.time()
    resultado = subprocess.run(
        [str(caminho_worker)], input=entrada, capture_output=True,
        text=True, timeout=1800,
    )
    tempo = time.time() - inicio

    if resultado.returncode != 0:
        imprimir(f"ERRO: rs falhou (rc={resultado.returncode}): "
                 f"{resultado.stderr[-400:]}")
        return None

    linhas_saida = resultado.stdout.strip().split("\n")
    if len(linhas_saida) != len(sentencas):
        imprimir(f"  AVISO: {len(linhas_saida)} saídas para "
                 f"{len(sentencas)} entradas")

    ipas = []
    for linha in linhas_saida:
        try:
            resp = json.loads(linha)
            sentencas_saida = resp.get("sentences", [])
            ipa = " ".join(s["phonemes"] for s in sentencas_saida)
            ipas.append(ipa)
        except Exception:
            ipas.append(None)

    imprimir(f"→ RS: {tempo:.2f}s")
    return ipas


def espeak_sentenca(sentenca):
    try:
        r = subprocess.run(
            ["espeak-ng", "-v", "pt-br", "--ipa=3", "-q", sentenca],
            capture_output=True, text=True, timeout=30,
        )
        return r.stdout.strip() or None
    except Exception:
        return None


def rodar_espeak_com_cache(sentencas, force=False):
    """Roda espeak em paralelo, com cache em disco."""
    if CAMINHO_ESPEAK_CACHE.exists() and not force:
        imprimir(f"→ Cache do espeak encontrado: {CAMINHO_ESPEAK_CACHE}")
        cache = json.loads(CAMINHO_ESPEAK_CACHE.read_text(encoding="utf-8"))
        if len(cache) >= len(sentencas) and cache[:len(sentencas)] == sentencas:
            imprimir(f"→ Cache válido para as {len(sentencas)} sentenças atuais")
            return None  # sinaliza para carregar depois
        imprimir(f"→ Cache desatualizado, regerando")

    imprimir(f"→ Rodando espeak em {len(sentencas)} sentenças "
             f"({QUANTIDADE_THREADS_ESPEAK} threads)...")
    inicio = time.time()
    resultados = [None] * len(sentencas)

    with ThreadPoolExecutor(max_workers=QUANTIDADE_THREADS_ESPEAK) as ex:
        futuros = {ex.submit(espeak_sentenca, s): i
                   for i, s in enumerate(sentencas)}
        concluidos = 0
        for fut in as_completed(futuros):
            i = futuros[fut]
            try:
                resultados[i] = fut.result()
            except Exception:
                resultados[i] = None
            concluidos += 1
            if concluidos % 500 == 0 or concluidos == len(sentencas):
                dec = time.time() - inicio
                imprimir(f"  [espeak] {concluidos}/{len(sentencas)} ({dec:.0f}s)")

    tempo = time.time() - inicio
    imprimir(f"→ espeak: {tempo:.2f}s")

    # Salvar cache: estrutura { sentencas: [...], espeak: [...] }
    CAMINHO_CACHE.mkdir(parents=True, exist_ok=True)
    CAMINHO_ESPEAK_CACHE.write_text(
        json.dumps({
            "sentencas": sentencas,
            "espeak": resultados,
        }, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    imprimir(f"→ Cache salvo em {CAMINHO_ESPEAK_CACHE}")
    return resultados


def carregar_espeak_cache(sentencas):
    """Carrega cache do espeak, validando se bate com as sentenças."""
    if not CAMINHO_ESPEAK_CACHE.exists():
        return None
    cache = json.loads(CAMINHO_ESPEAK_CACHE.read_text(encoding="utf-8"))
    if cache.get("sentencas") != sentencas:
        return None
    return cache.get("espeak")


# ---------------------------------------------------------------------------
# Comparação palavra a palavra
# ---------------------------------------------------------------------------

def comparar_palavras(ipas_a, ipas_b, sentencas):
    """Alinha por token e devolve lista de divergências."""
    divergencias = []
    total_tokens = 0
    iguais = 0
    estruturais = 0

    for idx, (a, b) in enumerate(zip(ipas_a, ipas_b)):
        if a is None or b is None:
            continue
        tokens_a = tokenizar_ipa(normalizar_ipa(a))
        tokens_b = tokenizar_ipa(normalizar_ipa(b))

        if len(tokens_a) != len(tokens_b):
            estruturais += 1
            continue

        tokens_texto = tokenizar_texto(sentencas[idx])

        for pos, (ta, tb) in enumerate(zip(tokens_a, tokens_b)):
            total_tokens += 1
            if ta == tb:
                iguais += 1
            else:
                palavra = (tokens_texto[pos]
                           if pos < len(tokens_texto) else f"[{pos}]")
                divergencias.append({
                    "indice": idx,
                    "sentenca": sentencas[idx],
                    "palavra": palavra.lower(),
                    "ipa_a": ta,
                    "ipa_b": tb,
                })

    return {
        "total": total_tokens,
        "match": iguais,
        "estruturais": estruturais,
        "difs": divergencias,
    }


# ---------------------------------------------------------------------------
# Análise de homógrafos
# ---------------------------------------------------------------------------

def carregar_homografos():
    if not CAMINHO_HOMOGRAFOS.exists():
        return {}
    return json.loads(CAMINHO_HOMOGRAFOS.read_text(encoding="utf-8"))


def avaliar_homografos(difs_rs_espeak, sentencas, homografos):
    """
    Para cada divergência cuja palavra-alvo tem regra em homograph_rules,
    conta como erro. Também conta quantas vezes o pipeline acertou.
    """
    palavras_hom = set(homografos.keys())

    # Contagem global por palavra
    total_ocorr = Counter()
    total_acertos = Counter()
    total_erros = Counter()
    exemplos_erro = defaultdict(list)

    # Percorrer todas as sentenças para contar TODAS as ocorrências
    # de homógrafos (não só as divergentes). Isso requer re-alinhar
    # rs vs espeak, mas como só queremos a contagem de acertos,
    # basta varrer os tokens IPA de cada sentença.
    # Simplificação: contamos as divergências do RS vs espeak
    # por palavra-alvo como erros. A contagem de acertos é obtida
    # pela diferença de ocorrências totais no corpus (calculada
    # separadamente abaixo).

    # Ocorrências totais de cada palavra-alvo no corpus
    for sent in sentencas:
        for t in tokenizar_texto(sent):
            base = t.lower().strip("'-")
            if base in palavras_hom:
                total_ocorr[base] += 1

    # Erros: cada divergência cuja palavra está em `palavras_hom`
    for d in difs_rs_espeak:
        p = d["palavra"]
        if p in palavras_hom:
            total_erros[p] += 1
            if len(exemplos_erro[p]) < 3:
                exemplos_erro[p].append({
                    "sentenca": d["sentenca"],
                    "rs": d["ipa_a"],
                    "espeak": d["ipa_b"],
                })

    # Acertos = ocorrências - erros (só vale se erro <= ocorrência)
    for p in palavras_hom:
        erros = total_erros.get(p, 0)
        ocorr = total_ocorr.get(p, 0)
        total_acertos[p] = max(0, ocorr - erros)

    return {
        "total_ocorr": total_ocorr,
        "acertos": total_acertos,
        "erros": total_erros,
        "exemplos_erro": exemplos_erro,
    }


# ---------------------------------------------------------------------------
# Distribuição de erros por sentença
# ---------------------------------------------------------------------------

def distribuicao_erros(difs):
    """Agrupa divergências por sentença."""
    por_sentenca = Counter()
    for d in difs:
        por_sentenca[d["indice"]] += 1
    return por_sentenca


def formatar_distribuicao(por_sentenca, total_sentencas):
    """Agrupa em buckets 0, 1, 2, 3, 4, 5+."""
    buckets = Counter()
    for i in range(total_sentencas):
        n = por_sentenca.get(i, 0)
        if n >= 5:
            buckets["5+"] += 1
        else:
            buckets[str(n)] += 1

    linhas = []
    for k in ["0", "1", "2", "3", "4", "5+"]:
        n = buckets.get(k, 0)
        pct = 100 * n / max(total_sentencas, 1)
        linhas.append((k, n, pct))
    return linhas, buckets


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("sentencas")
    ap.add_argument("--limite", type=int, default=0)
    ap.add_argument("--force-espeak", action="store_true",
                    help="Ignora o cache do espeak e regera")
    args = ap.parse_args()

    caminho = Path(args.sentencas).resolve()
    if not caminho.is_file():
        imprimir(f"ERRO: {caminho} não encontrado")
        sys.exit(1)

    imprimir("=" * 72)
    imprimir("COMPARAÇÃO — CORPUS DO BIFONIA")
    imprimir("=" * 72)
    imprimir(f"Sentenças: {caminho}")
    imprimir(f"Cache:     {CAMINHO_CACHE}")
    imprimir("")

    if not CAMINHO_WORKER.exists():
        imprimir(f"ERRO: worker não encontrado em {CAMINHO_WORKER}")
        imprimir("Compile com: cargo build --release --bin phonemizer-worker")
        sys.exit(1)

    texto = caminho.read_text(encoding="utf-8")
    sentencas_brutas = [l.strip() for l in texto.split("\n") if l.strip()]
    imprimir(f"→ {len(sentencas_brutas)} sentenças lidas")

    excecoes = carregar_excecoes()
    imprimir(f"→ {len(excecoes)} siglas nas exceções")

    sentencas, descartadas = filtrar_sentencas(sentencas_brutas, excecoes)
    imprimir(f"→ {len(sentencas)} sentenças após filtro "
             f"({descartadas} descartadas)")

    if args.limite > 0:
        sentencas = sentencas[:args.limite]
        imprimir(f"→ Limitado a {len(sentencas)}")

    if not sentencas:
        imprimir("ERRO: nenhuma sentença restou")
        sys.exit(1)

    # --- espeak com cache ---
    espeak_cache = None
    if not args.force_espeak:
        espeak_cache = carregar_espeak_cache(sentencas)
        if espeak_cache is not None:
            imprimir(f"→ Cache do espeak carregado ({len(espeak_cache)} entradas)")

    if espeak_cache is None:
        espeak_cache = rodar_espeak_com_cache(sentencas, force=args.force_espeak)

    ipas_es = espeak_cache

    # --- JS ---
    node_modules = encontrar_node_modules()
    if node_modules is None:
        imprimir("ERRO: vozz-js não encontrado")
        sys.exit(1)
    ipas_js = rodar_vozz_js(sentencas, node_modules, CAMINHO_CACHE)
    if ipas_js is None:
        sys.exit(1)

    # --- RS ---
    ipas_rs = rodar_vozz_rs(sentencas, CAMINHO_WORKER)
    if ipas_rs is None:
        sys.exit(1)

    # --- Comparação palavra a palavra ---
    imprimir("")
    imprimir("=" * 72)
    imprimir("PALAVRA A PALAVRA")
    imprimir("=" * 72)

    r_js_rs = comparar_palavras(ipas_js, ipas_rs, sentencas)
    r_rs_es = comparar_palavras(ipas_rs, ipas_es, sentencas)
    r_js_es = comparar_palavras(ipas_js, ipas_es, sentencas)

    for nome, r in [
        ("vozz-js vs vozz-rs", r_js_rs),
        ("vozz-rs vs espeak", r_rs_es),
        ("vozz-js vs espeak", r_js_es),
    ]:
        pct = 100 * r["match"] / max(r["total"], 1)
        imprimir(f"  {nome:<22} {r['match']:>7}/{r['total']:<7} = "
                 f"{pct:.2f}%  (estruturais: {r['estruturais']})")

    # --- Homógrafos ---
    imprimir("")
    imprimir("=" * 72)
    imprimir("ANÁLISE DE HOMÓGRAFOS")
    imprimir("=" * 72)

    homografos = carregar_homografos()
    if not homografos:
        imprimir("  ⚠ homograph_rules_v2.json não encontrado")
    else:
        imprimir(f"  {len(homografos)} palavras com regra")
        aval = avaliar_homografos(r_rs_es["difs"], sentencas, homografos)

        total_ocorr = sum(aval["total_ocorr"].values())
        total_acertos = sum(aval["acertos"].values())
        total_erros = sum(aval["erros"].values())

        imprimir("")
        imprimir(f"  Ocorrências totais de homógrafos no corpus: {total_ocorr}")
        imprimir(f"  Acertos (RS == espeak): {total_acertos}")
        imprimir(f"  Erros (RS != espeak):   {total_erros}")
        if total_ocorr > 0:
            pct = 100 * total_acertos / total_ocorr
            imprimir(f"  Acurácia em homógrafos: {pct:.2f}%")
        imprimir("")

        imprimir("  Top 15 palavras com mais erros:")
        ordenadas = sorted(aval["erros"].items(),
                           key=lambda kv: -kv[1])
        for p, n in ordenadas[:15]:
            if n == 0:
                continue
            ocorr = aval["total_ocorr"][p]
            pct_err = 100 * n / max(ocorr, 1)
            imprimir(f"    {p:<18} {n:>4} erros / {ocorr:>4} ocorr "
                     f"({pct_err:.1f}%)")

    # --- Distribuição de erros por sentença ---
    imprimir("")
    imprimir("=" * 72)
    imprimir("DISTRIBUIÇÃO DE ERROS POR SENTENÇA")
    imprimir("=" * 72)

    for nome, r in [
        ("vozz-rs vs espeak", r_rs_es),
        ("vozz-js vs espeak", r_js_es),
    ]:
        imprimir("")
        imprimir(f"▸ {nome}")
        por_sentenca = distribuicao_erros(r["difs"])
        linhas, buckets = formatar_distribuicao(por_sentenca, len(sentencas))

        for k, n, pct in linhas:
            barra = "█" * int(pct / 2)
            imprimir(f"    {k:>3} erro(s): {n:>5} sentenças ({pct:>5.1f}%) {barra}")

        # Total de erros
        imprimir(f"    Total de erros:  {sum(por_sentenca.values())}")
        imprimir(f"    Total sentenças: {len(sentencas)}")

    imprimir("")
    imprimir("=" * 72)
    imprimir("FIM")
    imprimir("=" * 72)


if __name__ == "__main__":
    main()