#!/usr/bin/env python3
"""
compare_sentences.py

Compara vozz-js x vozz-rs x espeak-ng em sentenças completas.

Entrada: sentences.txt (uma sentença por linha).

Filtragem: pula qualquer sentença que contenha:
  - tokens puramente numéricos;
  - tokens que precisam de normalização (abreviações, símbolos, etc.);
  - siglas sem vogal (exceptions.json).

Para cada sentença, fonemiza com três motores e compara em dois níveis:
  1. Sentença completa (string inteira normalizada).
  2. Palavra a palavra (tokeniza o IPA e alinha por posição).

As divergências palavra a palavra são agrupadas por categoria para
detectar padrões sistemáticos.

Uso:
  python3 compare_sentences.py sentences.txt
  python3 compare_sentences.py sentences.txt --limite 100
  python3 compare_sentences.py sentences.txt --max-exemplos 10
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

def imprimir(mensagem):
    print(mensagem, flush=True)


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
    """Divide o IPA em tokens, removendo pontuação das bordas."""
    if not ipa:
        return []
    tokens = ipa.split()
    limpos = []
    for token in tokens:
        limpo = RE_PONTUACAO_BORDA.sub("", token)
        if limpo:
            limpos.append(limpo)
    return limpos


def eh_puramente_numerico(palavra):
    return bool(palavra) and all(c.isdigit() and c.isascii() for c in palavra)


def eh_sigla_sem_vogal(palavra):
    minuscula = palavra.lower()
    tamanho = len(minuscula)
    if not (2 <= tamanho <= 6):
        return False
    if not minuscula.isalpha():
        return False
    return not any(c in VOGAIS_PORTUGUES for c in minuscula)


def precisa_normalizar(palavra):
    if any(not c.isalpha() for c in palavra):
        return True
    if palavra.lower() in ABREVIACOES:
        return True
    if eh_sigla_sem_vogal(palavra):
        return True
    return False


def classificar_divergencia(a, b):
    if a.replace("ˌ", "") == b.replace("ˌ", ""):
        return "acento-secundario"
    if a.replace("ˈ", "") == b.replace("ˈ", ""):
        return "acento-primario"
    if a.replace("ˈ", "").replace("ˌ", "") == b.replace("ˈ", "").replace("ˌ", ""):
        return "posicao-acento"
    if len(a) == len(b):
        diferencas = sum(1 for x, y in zip(a, b) if x != y)
        if diferencas == 1:
            return "1-char-diff"
        if diferencas <= 3:
            return f"{diferencas}-char-diff"
        return "varios-chars-diff"
    diferenca_tamanho = abs(len(a) - len(b))
    if diferenca_tamanho == 1:
        return "1-char-len"
    if diferenca_tamanho <= 3:
        return f"{diferenca_tamanho}-char-len"
    return "estrutural"


def carregar_excecoes():
    caminho = CAMINHO_CACHE / "exceptions.json"
    if not caminho.exists():
        return set()
    try:
        return set(json.loads(caminho.read_text(encoding="utf-8")))
    except Exception:
        return set()


def filtrar_sentencas(sentencas, excecoes):
    """Remove sentenças que contêm tokens que precisam normalização,
    tokens numéricos, ou siglas sem vogal em `excecoes`."""
    filtradas = []
    descartadas = 0
    for sentenca in sentencas:
        tokens = tokenizar_texto(sentenca)
        pular = False
        for token in tokens:
            limpo = token.lower().strip("'-")
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
            continue
        filtradas.append(sentenca)
    return filtradas, descartadas


# ---------------------------------------------------------------------------
# Motores
# ---------------------------------------------------------------------------

def encontrar_node_modules():
    for diretorio in CAMINHOS_NODE_MODULES:
        vozz = diretorio / "@pedrobef" / "vozz"
        if vozz.exists() and (vozz / "package.json").exists():
            return diretorio
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
    imprimir(f"→ Rodando vozz-g2p-js em {len(sentencas)} sentenças...")
    vozz = diretorio_node_modules / "@pedrobef" / "vozz"
    candidatos_entrypoint = [
        vozz / "src" / "index.js",
        vozz / "src" / "g2p" / "index.js",
        vozz / "g2p" / "index.js",
        vozz / "index.js",
    ]
    entrypoint = next((e for e in candidatos_entrypoint if e.exists()), None)
    if entrypoint is None:
        imprimir("  ERRO: entrypoint não encontrado")
        return None

    runner = diretorio_cache / "run_js_sentencas.mjs"
    runner.write_text(SCRIPT_RUNNER_JS, encoding="utf-8")
    caminho_sentencas = diretorio_cache / "sentencas_js.json"
    caminho_sentencas.write_text(
        json.dumps(sentencas, ensure_ascii=False), encoding="utf-8"
    )
    caminho_saida = diretorio_cache / "js_sentencas.json"

    inicio = time.time()
    processo = subprocess.Popen(
        ["node", str(runner), str(entrypoint),
         str(caminho_sentencas), str(caminho_saida)],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        text=True, cwd=str(diretorio_cache),
    )
    for linha in processo.stderr:
        linha = linha.rstrip()
        if linha:
            imprimir(f"  [js] {linha}")
    processo.wait()
    tempo_total = time.time() - inicio
    if processo.returncode != 0:
        imprimir(f"ERRO: js falhou (rc={processo.returncode})")
        return None
    imprimir(f"→ JS: {tempo_total:.2f}s")
    return json.loads(caminho_saida.read_text(encoding="utf-8"))


def rodar_vozz_rs(sentencas, caminho_worker):
    imprimir(f"→ Rodando vozz-g2p-rs em {len(sentencas)} sentenças...")
    linhas = []
    for sentenca in sentencas:
        requisicao = {
            "action": "process",
            "text": sentenca,
            "voice": "",
            "overrides": {},
        }
        linhas.append(json.dumps(requisicao, ensure_ascii=False))
    entrada = "\n".join(linhas) + "\n"

    inicio = time.time()
    resultado = subprocess.run(
        [str(caminho_worker)],
        input=entrada,
        capture_output=True,
        text=True,
        timeout=1800,
    )
    tempo_total = time.time() - inicio

    if resultado.returncode != 0:
        imprimir(f"ERRO: rs falhou (rc={resultado.returncode}): {resultado.stderr[-400:]}")
        return None

    linhas_saida = resultado.stdout.strip().split("\n")
    if len(linhas_saida) != len(sentencas):
        imprimir(f"  AVISO: {len(linhas_saida)} saídas para {len(sentencas)} entradas")

    ipas = []
    for linha in linhas_saida:
        try:
            resposta = json.loads(linha)
            sentencas_saida = resposta.get("sentences", [])
            ipa = " ".join(s["phonemes"] for s in sentencas_saida)
            ipas.append(ipa)
        except Exception:
            ipas.append(None)

    imprimir(f"→ RS: {tempo_total:.2f}s")
    return ipas


def espeak_sentenca(sentenca):
    try:
        resultado = subprocess.run(
            ["espeak-ng", "-v", "pt-br", "--ipa=3", "-q", sentenca],
            capture_output=True, text=True, timeout=30,
        )
        return resultado.stdout.strip() or None
    except Exception:
        return None


def rodar_espeak(sentencas):
    imprimir(f"→ Rodando espeak-ng em {len(sentencas)} sentenças "
             f"({QUANTIDADE_THREADS_ESPEAK} threads)...")
    inicio = time.time()
    resultados = [None] * len(sentencas)

    with ThreadPoolExecutor(max_workers=QUANTIDADE_THREADS_ESPEAK) as executor:
        futuros = {
            executor.submit(espeak_sentenca, s): indice
            for indice, s in enumerate(sentencas)
        }
        concluidos = 0
        for futuro in as_completed(futuros):
            indice = futuros[futuro]
            try:
                resultados[indice] = futuro.result()
            except Exception:
                resultados[indice] = None
            concluidos += 1
            if concluidos % 500 == 0 or concluidos == len(sentencas):
                decorrido = time.time() - inicio
                imprimir(f"  [espeak] {concluidos}/{len(sentencas)} ({decorrido:.0f}s)")

    tempo_total = time.time() - inicio
    imprimir(f"→ espeak: {tempo_total:.2f}s")
    return resultados


# ---------------------------------------------------------------------------
# Comparação
# ---------------------------------------------------------------------------

def comparar_sentencas(ipas_a, ipas_b, nome_a, nome_b):
    """Compara sentença por sentença."""
    total = 0
    iguais = 0
    divergencias = []

    for indice, (a, b) in enumerate(zip(ipas_a, ipas_b)):
        if a is None or b is None:
            continue
        total += 1
        if normalizar_ipa(a) == normalizar_ipa(b):
            iguais += 1
        else:
            divergencias.append((indice, normalizar_ipa(a), normalizar_ipa(b)))

    return {
        "name": f"{nome_a} vs {nome_b}",
        "total": total,
        "match": iguais,
        "difs": divergencias,
    }


def comparar_palavras(ipas_a, ipas_b, nome_a, nome_b, sentencas):
    """Compara palavra a palavra dentro das sentenças.

    Só considera sentenças onde as duas versões têm o mesmo número de
    tokens IPA. Caso contrário, marca a sentença inteira como estrutural.
    """
    divergencias = []
    total_tokens = 0
    iguais = 0
    estruturais = 0

    for indice, (a, b) in enumerate(zip(ipas_a, ipas_b)):
        if a is None or b is None:
            continue
        tokens_a = tokenizar_ipa(normalizar_ipa(a))
        tokens_b = tokenizar_ipa(normalizar_ipa(b))

        if len(tokens_a) != len(tokens_b):
            estruturais += 1
            continue

        tokens_texto = tokenizar_texto(sentencas[indice])

        for posicao, (ta, tb) in enumerate(zip(tokens_a, tokens_b)):
            total_tokens += 1
            if ta == tb:
                iguais += 1
            else:
                palavra = (
                    tokens_texto[posicao]
                    if posicao < len(tokens_texto)
                    else f"[{posicao}]"
                )
                divergencias.append((palavra, ta, tb, sentencas[indice]))

    return {
        "name": f"{nome_a} vs {nome_b}",
        "total": total_tokens,
        "match": iguais,
        "estruturais": estruturais,
        "difs": divergencias,
    }


def agrupar_por_categoria(divergencias, limite_exemplos=5):
    categorias = Counter()
    exemplos = defaultdict(list)

    for palavra, a, b, _sentenca in divergencias:
        categoria = classificar_divergencia(a, b)
        categorias[categoria] += 1
        if len(exemplos[categoria]) < limite_exemplos:
            exemplos[categoria].append((palavra, a, b))

    return categorias, exemplos


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(
        description="Compara motores em sentenças completas."
    )
    parser.add_argument("sentencas", help="Caminho do sentences.txt")
    parser.add_argument("--limite", type=int, default=0,
                        help="Limita o número de sentenças (0 = todas)")
    parser.add_argument("--diretorio-cache", default=None)
    parser.add_argument("--max-exemplos", type=int, default=5)
    argumentos = parser.parse_args()

    caminho_sentencas = Path(argumentos.sentencas).resolve()
    if not caminho_sentencas.is_file():
        imprimir(f"ERRO: arquivo não encontrado: {caminho_sentencas}")
        sys.exit(1)

    diretorio_cache = (
        Path(argumentos.diretorio_cache).resolve()
        if argumentos.diretorio_cache
        else CAMINHO_CACHE
    )
    diretorio_cache.mkdir(parents=True, exist_ok=True)

    imprimir("=" * 60)
    imprimir("COMPARAÇÃO POR SENTENÇA")
    imprimir("=" * 60)
    imprimir(f"Sentenças: {caminho_sentencas}")
    imprimir(f"Cache:     {diretorio_cache}")
    imprimir("")

    if not CAMINHO_WORKER.exists():
        imprimir(f"ERRO: worker não encontrado em {CAMINHO_WORKER}")
        imprimir("Compile com: cargo build --release --bin phonemizer-worker")
        sys.exit(1)

    texto = caminho_sentencas.read_text(encoding="utf-8")
    sentencas_brutas = [
        linha.strip() for linha in texto.split("\n") if linha.strip()
    ]
    imprimir(f"→ {len(sentencas_brutas)} sentenças lidas")

    excecoes = carregar_excecoes()
    imprimir(f"→ {len(excecoes)} siglas nas exceções")

    sentencas, descartadas = filtrar_sentencas(sentencas_brutas, excecoes)
    imprimir(f"→ {len(sentencas)} sentenças após filtro "
             f"({descartadas} descartadas por normalização/exceções)")

    if argumentos.limite > 0:
        sentencas = sentencas[: argumentos.limite]
        imprimir(f"→ Limitado a {len(sentencas)} sentenças")

    if not sentencas:
        imprimir("ERRO: nenhuma sentença restou após o filtro.")
        sys.exit(1)

    diretorio_node_modules = encontrar_node_modules()
    if diretorio_node_modules is None:
        imprimir("ERRO: vozz-js não encontrado. Rode compare_vozz.py antes.")
        sys.exit(1)

    ipas_js = rodar_vozz_js(sentencas, diretorio_node_modules, diretorio_cache)
    if ipas_js is None:
        sys.exit(1)

    ipas_rs = rodar_vozz_rs(sentencas, CAMINHO_WORKER)
    if ipas_rs is None:
        sys.exit(1)

    ipas_es = rodar_espeak(sentencas)

    (diretorio_cache / "sentencas_resultados.json").write_text(
        json.dumps({
            "sentencas": sentencas,
            "js": ipas_js,
            "rs": ipas_rs,
            "espeak": ipas_es,
        }, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )

    imprimir("")
    imprimir("=" * 72)
    imprimir("SENTENÇA COMPLETA")
    imprimir("=" * 72)

    pares_sentenca = [
        comparar_sentencas(ipas_js, ipas_rs, "vozz-js", "vozz-rs"),
        comparar_sentencas(ipas_rs, ipas_es, "vozz-rs", "espeak"),
        comparar_sentencas(ipas_js, ipas_es, "vozz-js", "espeak"),
    ]
    for par in pares_sentenca:
        if par["total"]:
            percentual = par["match"] / par["total"] * 100
            imprimir(
                f"  {par['name']:<20} {par['match']:>6}/{par['total']:<6} = "
                f"{percentual:.2f}%"
            )

    imprimir("")
    imprimir("=" * 72)
    imprimir("PALAVRA A PALAVRA (dentro das sentenças)")
    imprimir("=" * 72)

    pares_palavra = [
        comparar_palavras(ipas_js, ipas_rs, "vozz-js", "vozz-rs", sentencas),
        comparar_palavras(ipas_rs, ipas_es, "vozz-rs", "espeak", sentencas),
        comparar_palavras(ipas_js, ipas_es, "vozz-js", "espeak", sentencas),
    ]
    for par in pares_palavra:
        if par["total"]:
            percentual = par["match"] / par["total"] * 100
            imprimir(
                f"  {par['name']:<20} {par['match']:>6}/{par['total']:<6} = "
                f"{percentual:.2f}%  "
                f"(sentenças estruturais: {par['estruturais']})"
            )

    for par in pares_palavra:
        if not par["difs"]:
            continue
        imprimir("")
        imprimir("=" * 72)
        imprimir(f"PADRÕES — {par['name']}")
        imprimir("=" * 72)
        categorias, exemplos = agrupar_por_categoria(
            par["difs"], argumentos.max_exemplos
        )
        total_difs = len(par["difs"])
        for categoria, quantidade in categorias.most_common():
            percentual = quantidade / total_difs * 100
            imprimir(f"  {categoria:<25} {quantidade:>7}  ({percentual:>5.1f}%)")
            for palavra, a, b in exemplos[categoria]:
                a_disp = a if len(a) <= 30 else a[:27] + "..."
                b_disp = b if len(b) <= 30 else b[:27] + "..."
                imprimir(f"      {palavra:<20} {a_disp:<32} → {b_disp}")
            imprimir("")


if __name__ == "__main__":
    main()
