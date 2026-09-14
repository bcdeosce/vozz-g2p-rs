#!/usr/bin/env python3
"""
testar_regras_por_palavra.py

Para cada palavra, mede se o IPA final muda com o contexto.

Se SIM → a palavra tem uma regra fonológica (aplicar sândi resolve).
Se NÃO → a palavra é lexical (o IPA é fixo; entra no lexicon).

Cruza também com a origem no pipeline: a palavra vem do
`cliticos_contexto`? Do `lexico_espeak_contexto.json`? Do
`lexico_espeak.json`? Isso permite identificar onde o sândi
não está sendo aplicado.

Uso:
  python3 testar_regras_por_palavra.py
  python3 testar_regras_por_palavra.py --palavra de
  python3 testar_regras_por_palavra.py --min-ocorrencias 20
"""

import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path


DIRETORIO = Path(__file__).resolve().parent
CAMINHO_CACHE = DIRETORIO / "cache"
CAMINHO_DIVERGENCIAS = CAMINHO_CACHE / "divergencias_contexto_v2.json"


# Features que podem condicionar o sândi
FEATURES_CONTEXTUAIS = [
    "proxima_tipo",
    "proxima_inicial",
    "anterior_tipo",
    "anterior_final",
    "posicao_rel",
    "tem_pausa_depois",
    "tem_pausa_antes",
]


def imprimir(msg):
    print(msg, flush=True)


# ---------------------------------------------------------------------------
# Análise por palavra
# ---------------------------------------------------------------------------

def analisar_palavra(palavra, ocorrencias):
    """
    Para uma palavra, mede se o IPA muda com o contexto.

    Devolve dict com:
      - ipa_rs_unico: bool (RS sempre produz mesmo IPA?)
      - ipa_es_unico: bool (espeak sempre produz mesmo IPA?)
      - melhor_feature: qual feature melhor separa os IPAs do espeak
      - separacao: fração das ocorrências em que a feature muda o IPA
    """
    n = len(ocorrencias)

    ipas_rs = Counter(o["ipa_rs"] for o in ocorrencias)
    ipas_es = Counter(o["ipa_espeak"] for o in ocorrencias)

    rs_unico = len(ipas_rs) == 1
    es_unico = len(ipas_es) == 1

    # Testa cada feature: ela separa os IPAs do espeak?
    resultados_feature = {}
    for feature in FEATURES_CONTEXTUAIS:
        grupos = defaultdict(Counter)
        for o in ocorrencias:
            val = o.get(feature)
            if val is None:
                continue
            grupos[val][o["ipa_espeak"]] += 1

        if not grupos:
            continue

        # Para cada bucket, qual o IPA dominante?
        ipa_dominante_por_bucket = {}
        for val, cnt in grupos.items():
            ipa_top, _ = cnt.most_common(1)[0]
            ipa_dominante_por_bucket[val] = ipa_top

        # Quantos IPAs distintos entre os buckets?
        n_ipas = len(set(ipa_dominante_por_bucket.values()))

        # Se n_ipas > 1, a feature discrimina
        if n_ipas > 1:
            # Qual a "pureza" média dos buckets?
            total_acertos = 0
            total = 0
            for val, cnt in grupos.items():
                total += sum(cnt.values())
                _, freq = cnt.most_common(1)[0]
                total_acertos += freq

            precisao = total_acertos / total
            resultados_feature[feature] = {
                "n_buckets": len(grupos),
                "n_ipas": n_ipas,
                "precisao": precisao,
                "ipa_por_bucket": dict(ipa_dominante_por_bucket),
            }

    # Escolhe a melhor feature (mais buckets, precisão alta)
    melhor_feature = None
    melhor_score = -1.0
    for feat, info in resultados_feature.items():
        # Score: precisão × log(n_buckets + 1)
        import math
        score = info["precisao"] * math.log(info["n_buckets"] + 1)
        if score > melhor_score:
            melhor_score = score
            melhor_feature = feat

    return {
        "n_ocorrencias": n,
        "rs_unico": rs_unico,
        "es_unico": es_unico,
        "ipas_rs": dict(ipas_rs.most_common(5)),
        "ipas_es": dict(ipas_es.most_common(5)),
        "resultados_feature": resultados_feature,
        "melhor_feature": melhor_feature,
    }


# ---------------------------------------------------------------------------
# Relatório
# ---------------------------------------------------------------------------

def relatar_palavra(palavra, dados, verboso=False):
    L = []
    add = L.append

    add(f"▸ {palavra}  ({dados['n_ocorrencias']} ocorrências)")
    add("")

    # IPAs
    ipas_rs_str = ", ".join(f"{ipa}({n})" for ipa, n in dados["ipas_rs"].items())
    ipas_es_str = ", ".join(f"{ipa}({n})" for ipa, n in dados["ipas_es"].items())

    add(f"    RS:     {ipas_rs_str}")
    add(f"    espeak: {ipas_es_str}")
    add("")

    # Veredito
    if dados["rs_unico"] and dados["es_unico"]:
        add("    → LEXICAL: IPA fixo, sem variação contextual.")
    elif not dados["rs_unico"] and not dados["es_unico"]:
        add("    → AMBÍGUO: ambos os sistemas variam.")
    elif dados["rs_unico"] and not dados["es_unico"]:
        add("    → REGRA: espeak varia, RS não. RS precisa aprender a regra.")
    else:
        add("    → SÂNDI INVERSO: RS varia, espeak não. RS precisa remover a regra.")

    add("")

    # Melhor feature
    if dados["melhor_feature"]:
        feat = dados["melhor_feature"]
        info = dados["resultados_feature"][feat]
        add(f"    Melhor feature: {feat}")
        add(f"      buckets={info['n_buckets']}  "
            f"IPAs distintos={info['n_ipas']}  "
            f"precisão={info['precisao']:.1%}")
        for val, ipa in info["ipa_por_bucket"].items():
            add(f"      {val:<20} → {ipa}")
        add("")

        if verboso:
            for f, i in dados["resultados_feature"].items():
                if f == feat:
                    continue
                add(f"    Outras features:")
                add(f"      {f:<25} buckets={i['n_buckets']}  "
                    f"IPAs={i['n_ipas']}  precisão={i['precisao']:.1%}")
            add("")

    add("")

    return "\n".join(L)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--palavra", default=None,
                    help="Analisa só esta palavra")
    ap.add_argument("--min-ocorrencias", type=int, default=10,
                    help="Só reporta palavras com N ocorrências (default: 10)")
    ap.add_argument("--top", type=int, default=30,
                    help="Top N palavras a reportar (default: 30)")
    ap.add_argument("--verboso", action="store_true",
                    help="Mostra todas as features testadas")
    ap.add_argument("--saida", default="cache/relatorio_regras_por_palavra.txt")
    args = ap.parse_args()

    if not CAMINHO_DIVERGENCIAS.exists():
        imprimir(f"ERRO: {CAMINHO_DIVERGENCIAS} não encontrado")
        return 1

    imprimir(f"→ Carregando {CAMINHO_DIVERGENCIAS}")
    itens = json.loads(CAMINHO_DIVERGENCIAS.read_text(encoding="utf-8"))
    imprimir(f"→ {len(itens)} divergências")

    # Agrupa por palavra
    por_palavra = defaultdict(list)
    for item in itens:
        por_palavra[item["palavra"]].append(item)

    imprimir(f"→ {len(por_palavra)} palavras distintas")
    imprimir("")

    L = []
    add = L.append

    add("=" * 78)
    add("ANÁLISE POR PALAVRA")
    add("=" * 78)
    add("")

    if args.palavra:
        alvos = [args.palavra] if args.palavra in por_palavra else []
        if not alvos:
            imprimir(f"Palavra '{args.palavra}' não encontrada")
            return 1
    else:
        # Só palavras com >= min_ocorrencias, ordenadas por frequência
        alvos = [
            p for p, ocs in por_palavra.items()
            if len(ocs) >= args.min_ocorrencias
        ]
        alvos.sort(key=lambda p: -len(por_palavra[p]))
        alvos = alvos[:args.top]

    contagem = Counter()

    for palavra in alvos:
        ocorrencias = por_palavra[palavra]
        dados = analisar_palavra(palavra, ocorrencias)
        texto = relatar_palavra(palavra, dados, verboso=args.verboso)
        add(texto)

        # Contabiliza
        if dados["rs_unico"] and dados["es_unico"]:
            contagem["lexical"] += 1
        elif not dados["rs_unico"] and not dados["es_unico"]:
            contagem["ambiguo"] += 1
        elif dados["rs_unico"] and not dados["es_unico"]:
            contagem["regra_faltando"] += 1
        else:
            contagem["sandhi_inverso"] += 1

    # Sumário
    add("=" * 78)
    add("SUMÁRIO")
    add("=" * 78)
    add("")
    total = sum(contagem.values())
    add(f"  Palavras analisadas: {total}")
    add("")
    add(f"  Lexicais:              {contagem['lexical']:>4} "
        f"({100*contagem['lexical']/total:.1f}%)")
    add(f"  Ambíguas:              {contagem['ambiguo']:>4} "
        f"({100*contagem['ambiguo']/total:.1f}%)")
    add(f"  Regra faltando:        {contagem['regra_faltando']:>4} "
        f"({100*contagem['regra_faltando']/total:.1f}%)")
    add(f"  Sândi inverso (bug):   {contagem['sandhi_inverso']:>4} "
        f"({100*contagem['sandhi_inverso']/total:.1f}%)")
    add("")
    add("  LEGENDA:")
    add("    Lexicais:        IPA fixo, sem regra. Vai para lexicon.")
    add("    Ambíguas:        Ambos variam. Investigar caso a caso.")
    add("    Regra faltando:  Espeak varia, RS não. Aplicar sândi resolve.")
    add("    Sândi inverso:   RS varia, espeak não. Remover sândi.")

    texto = "\n".join(L)
    imprimir(texto)

    Path(args.saida).write_text(texto, encoding="utf-8")
    imprimir(f"\n→ Salvo em {args.saida}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())