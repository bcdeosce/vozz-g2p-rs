#!/usr/bin/env python3
"""
gerar_lexicon_homografos.py

Gera `lexicon_homografos.json` — mapa (palavra, sentido) → IPA do espeak.

O IPA armazenado é a **forma base** (sem sândi de glide, sem
dessonorização, sem tap). O sândi é aplicado em runtime, quando a
palavra seguinte é conhecida.

Se dois sentidos colapsam para a mesma forma base, ambos ficam no
lexicon com o mesmo valor. Nenhum filtro de colapso é aplicado.

Entrada:  homograph_rules_v2.json + bifonia_pt_homographs_no_ipa.jsonl
Cache:    cache/espeak_sentences_bifonia.json (obrigatório)
Saída:    lexicon_homografos.json

Uso:
  python3 gerar_lexicon_homografos.py \\
    --dataset bifonia_pt_homographs_no_ipa.jsonl \\
    --regras  homograph_rules_v2.json \\
    --saida   cache/lexicon_homografos.json \\
    --cache   cache
"""

import argparse
import json
import re
import sys
import unicodedata
from collections import Counter, defaultdict
from pathlib import Path


RE_TOKEN = re.compile(r"\w+", re.UNICODE)
RE_PONT_BORDA = re.compile(
    r"^[;:,.!?¡¿—…\"«»“”(){}]+|[;:,.!?¡¿—…\"«»“”(){}]+$"
)
NOME_CACHE_PADRAO = "espeak_sentences_bifonia.json"


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


def tokenizar_ipa(ipa):
    if not ipa:
        return []
    return [t for t in (RE_PONT_BORDA.sub("", x) for x in ipa.split()) if t]


def tokenizar_texto(t):
    return RE_TOKEN.findall(t)


def base_ipa(ipa):
    """
    Remove a variação de sândi final, deixando a forma base.

    Regras (idempotentes):
      - `z` final → `s` (dessonorização em coda)
      - `w` final → `ʊ` (glide antes de vogal)
      - `j` final → `y` (palatalização antes de vogal)
      - `ɾ` final → `r` (tap antes de vogal)

    Aplicar múltiplas vezes é seguro: aplicar sobre a forma base é no-op.
    """
    if not ipa:
        return ipa
    ultimo = ipa[-1]
    mapa = {
        "z": "s",
        "w": "ʊ",
        "j": "y",
        "ɾ": "r",
    }
    if ultimo in mapa:
        return ipa[:-1] + mapa[ultimo]
    return ipa


def alinhar(sentenca, ipa):
    palavras = tokenizar_texto(sentenca)
    tokens = tokenizar_ipa(normalizar_ipa(ipa))
    if len(palavras) != len(tokens):
        return None
    return list(zip(palavras, tokens))


# ---------------------------------------------------------------------------
# Cache do espeak-ng
# ---------------------------------------------------------------------------

def carregar_cache_espeak(caminho):
    """
    Carrega o cache do espeak.

    Formatos aceitos:
      - {"sentencas": [...], "espeak": [...]}  (compare_sentences_bifonia.py)
      - {sentenca: ipa, ...}                    (formato alternativo)

    Devolve `{sentenca: ipa}` ou `None` se o arquivo não existir
    ou estiver corrompido.
    """
    if not caminho.exists():
        imprimir(f"  ERRO: cache não encontrado em {caminho}")
        return None

    try:
        dados = json.loads(caminho.read_text(encoding="utf-8"))
    except Exception as erro:
        imprimir(f"  ERRO: falha ao ler cache {caminho}: {erro}")
        return None

    if isinstance(dados, dict) and "sentencas" in dados and "espeak" in dados:
        sentencas = dados["sentencas"]
        ipas = dados["espeak"]
        if len(sentencas) != len(ipas):
            imprimir(f"  ERRO: cache corrompido — {len(sentencas)} sentenças "
                     f"vs {len(ipas)} ipas")
            return None
        return {s: ipa for s, ipa in zip(sentencas, ipas)}

    if isinstance(dados, dict):
        return dados

    imprimir(f"  ERRO: cache em formato desconhecido: {type(dados)}")
    return None


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dataset", required=True,
                    help="JSONL do bifonia sem IPA")
    ap.add_argument("--regras", required=True,
                    help="homograph_rules_v2.json")
    ap.add_argument("--saida", default="cache/lexicon_homografos.json")
    ap.add_argument("--cache", default="cache",
                    help="Diretório do cache")
    ap.add_argument("--cache-espeak", default=None,
                    help="Caminho do cache do espeak. "
                         "Default: <cache>/espeak_sentences_bifonia.json")
    ap.add_argument("--min-ocorrencias", type=int, default=1,
                    help="Mínimo de exemplos por sentido (default: 1).")
    args = ap.parse_args()

    # --- Carrega o cache do espeak primeiro ---
    if args.cache_espeak:
        caminho_cache = Path(args.cache_espeak).resolve()
    else:
        caminho_cache = Path(args.cache).resolve() / NOME_CACHE_PADRAO

    imprimir(f"→ Carregando cache: {caminho_cache}")
    cache = carregar_cache_espeak(caminho_cache)
    if cache is None:
        sys.exit(1)
    imprimir(f"→ {len(cache)} sentenças no cache")

    # --- Palavras com regra ---
    regras = json.loads(Path(args.regras).read_text(encoding="utf-8"))
    palavras_alvo = set(regras.keys())
    imprimir(f"→ {len(palavras_alvo)} palavras com regra")

    # --- Lê JSONL, filtrando pelo cache ---
    exemplos_totais = 0
    exemplos_no_cache = 0
    exemplos_fora_cache = 0
    exemplos_sem_sentido = 0

    exemplos = []
    with open(args.dataset, encoding="utf-8") as f:
        for linha in f:
            if not linha.strip():
                continue
            try:
                item = json.loads(linha)
            except json.JSONDecodeError:
                continue
            exemplos_totais += 1

            w = (item.get("word") or "").lower().strip()
            s = (item.get("sense") or "").strip()
            sent = (item.get("sentence") or "").strip()

            if w not in palavras_alvo or not s or not sent:
                exemplos_sem_sentido += 1
                continue

            if sent not in cache:
                exemplos_fora_cache += 1
                continue

            exemplos.append((w, s, sent))
            exemplos_no_cache += 1

    imprimir("")
    imprimir("FILTRAGEM DO JSONL")
    imprimir(f"  → {exemplos_totais} exemplos totais no JSONL")
    imprimir(f"  → {exemplos_sem_sentido} sem palavra-alvo, sentido ou sentença")
    imprimir(f"  → {exemplos_fora_cache} sentenças fora do cache "
             f"(filtradas por normalização)")
    imprimir(f"  → {exemplos_no_cache} exemplos válidos (sentença no cache)")
    imprimir("")

    if not exemplos:
        imprimir("ERRO: nenhum exemplo válido sobrou")
        sys.exit(1)

    # --- Agregar por (word, sense) ---
    imprimir("→ Alinhando e acumulando IPA por (palavra, sentido)...")
    agregado = defaultdict(lambda: defaultdict(Counter))
    com_ipa = 0
    sem_alinhamento = 0
    sem_espeak = 0

    for w, s, sent in exemplos:
        ipa = cache.get(sent)
        if not ipa:
            sem_espeak += 1
            continue
        al = alinhar(sent, ipa)
        if al is None:
            sem_alinhamento += 1
            continue
        for pal, ipa_pal in al:
            if pal.lower() == w:
                agregado[w][s][ipa_pal] += 1
                com_ipa += 1
                break

    imprimir(f"→ {com_ipa} exemplos com IPA extraído")
    imprimir(f"→ {sem_alinhamento} sentenças descartadas por desalinhamento")
    imprimir(f"→ {sem_espeak} sentenças sem IPA no cache")

    # --- Construir o resultado final ---
    resultado = {}
    descartadas_poucas_ocorrencias = 0

    for w, sentidos in agregado.items():
        sentidos_validos = {}
        for s, contagem in sentidos.items():
            total = sum(contagem.values())
            if total < args.min_ocorrencias:
                descartadas_poucas_ocorrencias += 1
                continue
            ipa_top, freq = contagem.most_common(1)[0]
            sentidos_validos[s] = {
                "ipa": base_ipa(ipa_top),
                "confianca": freq / total,
                "ocorrencias": total,
            }
        if sentidos_validos:
            resultado[w] = sentidos_validos

    imprimir("")
    imprimir("FILTROS APLICADOS")
    imprimir(f"  → {descartadas_poucas_ocorrencias} sentidos com < "
             f"{args.min_ocorrencias} ocorrências")
    imprimir(f"  → {len(resultado)} palavras no lexicon final")

    # --- Salvar ---
    saida = Path(args.saida)
    saida.parent.mkdir(parents=True, exist_ok=True)
    with open(saida, "w", encoding="utf-8") as f:
        json.dump(resultado, f, ensure_ascii=False, indent=2)
    imprimir(f"→ Salvo em {saida}")

    # --- Amostra ---
    imprimir("")
    imprimir("AMOSTRA (top 20):")
    for w, sentidos in list(resultado.items())[:20]:
        imprimir(f"  {w}")
        for s, dados in sentidos.items():
            imprimir(f"    {s:<25} IPA={dados['ipa']:<20} "
                     f"conf={dados['confianca']:.2f} n={dados['ocorrencias']}")


if __name__ == "__main__":
    main()