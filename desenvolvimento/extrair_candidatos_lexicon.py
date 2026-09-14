#!/usr/bin/env python3
"""
extrair_candidatos_lexicon.py

Lê `sentencas_resultados.json` e devolve uma lista priorizada de
palavras onde o RS diverge do espeak, com o par (RS, espeak) e a
frequência. Útil para adicionar entradas em `lexicon_contexto.json`.

Uso:
  python3 extrair_candidatos_lexicon.py --min-ocorrencias 5
"""

import argparse
import json
import unicodedata
from collections import Counter, defaultdict
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor


CAMINHO_CACHE = Path("cache")
CAMINHO_RESULTADOS = CAMINHO_CACHE / "sentencas_resultados.json"

RE_PONT = None
import re
RE_PONT = re.compile(r"^[;:,.!?¡¿—…\"«»“”(){}]+|[;:,.!?¡¿—…\"«»“”(){}]+$")


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
    return [t for t in (RE_PONT.sub("", x) for x in ipa.split()) if t]


def tokenizar_texto(t):
    return re.findall(r"[\w'-]+", t, re.UNICODE)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--min-ocorrencias", type=int, default=3)
    ap.add_argument("--top", type=int, default=50)
    args = ap.parse_args()

    dados = json.loads(CAMINHO_RESULTADOS.read_text(encoding="utf-8"))
    sentencas = dados["sentencas"]
    ipas_rs = dados["rs"]
    ipas_es = dados["espeak"]

    # Contador: chave = (rs_ipa, es_ipa) → Counter(palavra)
    por_par = defaultdict(Counter)

    for idx, sentenca in enumerate(sentencas):
        if idx >= len(ipas_rs) or idx >= len(ipas_es):
            continue
        rs = ipas_rs[idx]
        es = ipas_es[idx]
        if not rs or not es:
            continue

        tokens_rs = tokenizar_ipa(normalizar_ipa(rs))
        tokens_es = tokenizar_ipa(normalizar_ipa(es))
        palavras = tokenizar_texto(sentenca)

        if len(tokens_rs) != len(tokens_es) or len(palavras) != len(tokens_rs):
            continue

        for pal, t_rs, t_es in zip(palavras, tokens_rs, tokens_es):
            if t_rs != t_es:
                chave = (t_rs, t_es)
                por_par[chave][pal.lower()] += 1

    # Ordena por frequência total
    ordenados = sorted(
        por_par.items(),
        key=lambda kv: -sum(kv[1].values())
    )

    print(f"# Top {args.top} pares (RS, espeak) — candidatos a lexicon")
    print()
    print(f"{'RS':<25} {'espeak':<25} {'total':>6}  palavras")
    print("-" * 100)

    for (rs, es), palavras in ordenados[:args.top]:
        total = sum(palavras.values())
        if total < args.min_ocorrencias:
            continue
        top_palavras = ", ".join(
            f"{p}({n})" for p, n in palavras.most_common(5)
        )
        print(f"{rs:<25} {es:<25} {total:>6}  {top_palavras}")

    # Sugestão: blocos JSON
    print()
    print("# Candidatos por palavra (palavra que aparece >5 vezes):")
    por_palavra = defaultdict(Counter)
    for (rs, es), palavras in por_par.items():
        for pal, n in palavras.items():
            por_palavra[pal][(rs, es)] += n

    # Palavras onde uma única correção resolve várias ocorrências
    print()
    print("# Correções de alta prioridade (palavra com >=10 ocorrências):")
    for pal, pares in sorted(por_palavra.items(), key=lambda kv: -sum(kv[1].values())):
        total = sum(pares.values())
        if total < 10:
            continue
        rs_es, freq = pares.most_common(1)[0]
        rs, es = rs_es
        print(f'  "{pal}":  RS={rs}  espeak={es}  ({freq} de {total})')


if __name__ == "__main__":
    main()