#!/usr/bin/env python3
"""
mapear_regra_palavra.py

Para cada palavra, roda o espeak em 4 contextos controlados:
  1. "X + palavra + água" (próxima = vogal)
  2. "X + palavra + casa" (próxima = consoante surda)
  3. "X + palavra + bola" (próxima = consoante sonora)
  4. "X + palavra"        (fim de sentença)

Compara os 4 IPAs e detecta:
  - Qual a regra de sândi da palavra
  - Se o RS está aplicando corretamente

Uso:
  python3 mapear_regra_palavra.py --palavra de
  python3 mapear_regra_palavra.py --palavra bebo
  python3 mapear_regra_palavra.py --palavra pode
"""

import argparse
import json
import re
import subprocess
import unicodedata
from collections import Counter
from pathlib import Path


RE_TOKEN = re.compile(r"\w+", re.UNICODE)
RE_PONT = re.compile(r"^[;:,.!?¡¿—…\"«»“”(){}]+|[;:,.!?¡¿—…\"«»“”(){}]+$")


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
    return [t for t in (RE_PONT.sub("", x) for x in ipa.split()) if t]


def espeak_ipa(sentenca):
    try:
        r = subprocess.run(
            ["espeak-ng", "-v", "pt-br", "--ipa=3", "-q", sentenca],
            capture_output=True, text=True, timeout=30,
        )
        return r.stdout.strip() or None
    except Exception:
        return None


def extrair_ipa_palavra(sentenca, ipa_sentenca, alvo):
    """Alinha e extrai o IPA da palavra alvo."""
    palavras = RE_TOKEN.findall(sentenca)
    tokens = tokenizar_ipa(normalizar_ipa(ipa_sentenca))
    if len(palavras) != len(tokens):
        return None
    for pal, ipa in zip(palavras, tokens):
        if pal.lower() == alvo:
            return ipa
    return None


def rodar_rs(sentenca, worker_path):
    """Roda o phonemizer-worker numa sentença."""
    req = {"action": "process", "text": sentenca, "voice": "", "overrides": {}}
    r = subprocess.run(
        [str(worker_path)],
        input=json.dumps(req) + "\n",
        capture_output=True, text=True, timeout=30,
    )
    if r.returncode != 0:
        return None
    try:
        resp = json.loads(r.stdout.strip().split("\n")[-1])
        sents = resp.get("sentences", [])
        if sents:
            return sents[0]["phonemes"]
    except Exception:
        pass
    return None


# Contextos controlados: (nome, template)
CONTEXTOS = [
    ("antes de vogal",        "Eu vejo {p} água"),
    ("antes de sonora",       "Eu vejo {p} bola"),
    ("antes de surda",        "Eu vejo {p} coisa"),
    ("fim de sentença",       "Eu vejo {p}"),
    ("início de sentença",    "{p} está aqui"),
    ("isolado",               "{p}"),
]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--palavra", required=True)
    ap.add_argument("--worker", default="target/release/phonemizer-worker")
    ap.add_argument("--cache", default="cache/espeak_sentences_bifonia.json")
    args = ap.parse_args()

    palavra = args.palavra.lower()
    worker_path = Path(args.worker).resolve()
    if not worker_path.exists():
        imprimir(f"ERRO: worker não encontrado em {worker_path}")
        return 1

    imprimir(f"→ Mapeando regra para '{palavra}'")
    imprimir("")

    imprimir(f"{'Contexto':<22} {'espeak':<22} {'RS':<22} Match")
    imprimir("-" * 72)

    for nome_ctx, template in CONTEXTOS:
        sentenca = template.format(p=palavra)
        ipa_es_full = espeak_ipa(sentenca)
        ipa_rs_full = rodar_rs(sentenca, worker_path)

        if not ipa_es_full or not ipa_rs_full:
            imprimir(f"{nome_ctx:<22} (erro ao rodar)")
            continue

        ipa_es = extrair_ipa_palavra(sentenca, ipa_es_full, palavra)
        ipa_rs = extrair_ipa_palavra(sentenca, ipa_rs_full, palavra)

        if ipa_es is None:
            imprimir(f"{nome_ctx:<22} (não alinhado)")
            continue

        ipa_rs_str = ipa_rs if ipa_rs else "?"
        match = "✓" if ipa_es == ipa_rs else "✗"

        imprimir(f"{nome_ctx:<22} {ipa_es:<22} {ipa_rs_str:<22} {match}")

    imprimir("")
    imprimir("ANÁLISE:")

    # Coleta de dados brutos
    resultados = []
    for nome_ctx, template in CONTEXTOS:
        sentenca = template.format(p=palavra)
        ipa_es_full = espeak_ipa(sentenca)
        ipa_rs_full = rodar_rs(sentenca, worker_path)
        if not ipa_es_full or not ipa_rs_full:
            continue
        ipa_es = extrair_ipa_palavra(sentenca, ipa_es_full, palavra)
        ipa_rs = extrair_ipе_palavra(sentenca, ipa_rs_full, palavra) if False else None
        # (o correto é chamar extrair_ipa_palavra)
        ipa_rs = extrair_ipa_palavra(sentenca, ipa_rs_full, palavra)
        if ipa_es:
            resultados.append((nome_ctx, ipa_es, ipa_rs))

    ipas_es = [r[1] for r in resultados]
    ipas_rs = [r[2] for r in resultados if r[2]]

    es_unico = len(set(ipas_es)) == 1
    rs_unico = len(set(ipas_rs)) == 1

    if es_unico and rs_unico:
        if ipas_es[0] == ipas_rs[0]:
            imprimir("  → REGRA CONSISTENTE: espeak e RS concordam em todos os contextos.")
        else:
            imprimir(f"  → ERRO SISTEMÁTICO:")
            imprimir(f"      espeak: {ipas_es[0]}")
            imprimir(f"      RS:     {ipas_rs[0]}")
            imprimir(f"      O RS precisa converter '{ipas_rs[0]}' → '{ipas_es[0]}'")
            imprimir(f"      Provavelmente é regra fixa (lexicon ou clitico).")

    elif es_unico and not rs_unico:
        imprimir("  → RS EXAGERA: espeak fixo, RS varia.")
        imprimir(f"      espeak: {ipas_es[0]}")
        imprimir(f"      RS varia: {sorted(set(ipas_rs))}")
        imprimir(f"      RS está aplicando sândi onde espeak não aplica.")

    elif not es_unico and rs_unico:
        imprimir("  → REGRA FALTANDO: espeak varia, RS fixo.")
        imprimir(f"      RS: {ipas_rs[0]}")
        imprimir(f"      espeak varia:")
        for ctx, ipa_es, _ in resultados:
            imprimir(f"        {ctx:<22} → {ipa_es}")
        imprimir(f"      RS precisa aprender a variar conforme o contexto.")

    else:
        imprimir("  → AMBÍGUO: ambos variam.")
        for ctx, ipa_es, ipa_rs in resultados:
            match = "✓" if ipa_es == ipa_rs else "✗"
            imprimir(f"      {ctx:<22} espeak={ipa_es:<20} RS={ipa_rs} {match}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())