#!/usr/bin/env python3
"""
extrair_regras_contextuais_v2.py

Extrai regras implementáveis em Rust a partir de:
  - `divergencias_contexto_v2.json` (features ricas do contexto)
  - `deepinfra_gatilhos_v2.json`    (classificação do LLM)

Cada regra tem o formato:

    SE <feature>=<valor> [E palavra ∈ P] ENTÃO ipa_alvo

Saída:
  - `relatorio_regras_v2.txt`      (relatório legível)
  - `regras_contextuais_v2.json`   (JSON estruturado para implementação)

Uso:
  python3 extrair_regras_contextuais_v2.py
  python3 extrair_regras_contextuais_v2.py --min-ocorrencias 3 --min-precisao 0.85
"""

import argparse
import json
import math
from collections import Counter, defaultdict
from pathlib import Path


DIRETORIO = Path(__file__).resolve().parent
CAMINHO_CACHE = DIRETORIO / "cache"
CAMINHO_DIVERGENCIAS = CAMINHO_CACHE / "divergencias_contexto_v2.json"
CAMINHO_GATILHOS = CAMINHO_CACHE / "deepinfra_gatilhos_v2.json"
CAMINHO_RELATORIO = CAMINHO_CACHE / "relatorio_regras_v2.txt"
CAMINHO_REGRAS = CAMINHO_CACHE / "regras_contextuais_v2.json"


VOGAIS = "aeiouáéíóúâêôãõàü"


def imprimir(msg):
    print(msg, flush=True)


# ---------------------------------------------------------------------------
# Categorização
# ---------------------------------------------------------------------------

def categorizar_letra(letra):
    if not letra:
        return "vazio"
    c = letra.lower()[0]
    if c in VOGAIS:
        return "vogal"
    if c in "bdgvzmnlr":
        return "sonora"
    if c in "ptkfsxʃ":
        return "surda"
    return "outra"


def bucket_posicao(posicao, total):
    if total <= 1:
        return "unico"
    rel = posicao / (total - 1)
    if rel < 0.2:
        return "inicio"
    if rel > 0.8:
        return "fim"
    return "meio"


def bucket_tamanho(total):
    if total <= 5:
        return "curta"
    if total <= 15:
        return "media"
    return "longa"


# ---------------------------------------------------------------------------
# Extração de features
# ---------------------------------------------------------------------------

FEATURES_CATEGORICAS = {
    "posicao_rel": lambda c: c.get("posicao_rel"),
    "anterior_class": lambda c: c.get("anterior_class"),
    "proxima_class": lambda c: c.get("proxima_class"),
    "proxima_tipo": lambda c: c.get("proxima_tipo"),
    "tipo_sintagma": lambda c: c.get("tipo_sintagma"),
    "posicao_sintagma": lambda c: c.get("posicao_sintagma"),
    "anterior_tipo": lambda c: categorizar_letra(c.get("anterior_final") or ""),
    "proxima_tipo2": lambda c: categorizar_letra(c.get("proxima_inicial") or ""),
    "posicao_bucket": lambda c: bucket_posicao(c.get("posicao", 0), c.get("total", 1)),
    "tamanho_bucket": lambda c: bucket_tamanho(c.get("total", 0)),
}

FEATURES_BOOLEANAS = {
    "tem_artigo_antes": "tem_artigo_antes",
    "tem_preposicao_antes": "tem_preposicao_antes",
    "verbo_antes": "verbo_antes",
    "verbo_depois": "verbo_depois",
    "auxiliar_antes": "auxiliar_antes",
    "auxiliar_depois": "auxiliar_depois",
    "tem_pausa_depois": "tem_pausa_depois",
    "tem_pausa_antes": "tem_pausa_antes",
}


def extrair_features(item):
    c = item.get("contexto", item)  # aceita ambos formatos
    feats = {}
    for nome, fn in FEATURES_CATEGORICAS.items():
        feats[nome] = fn(c)
    for nome, campo in FEATURES_BOOLEANAS.items():
        feats[nome] = "sim" if c.get(campo) else "nao"
    return feats


# ---------------------------------------------------------------------------
# Avaliação de feature
# ---------------------------------------------------------------------------

def avaliar_feature(grupo, feature_name, min_ocorrencias, min_precisao, baseline):
    por_valor = defaultdict(lambda: {"total": 0, "ipas": Counter()})
    for item in grupo:
        val = item["_features"].get(feature_name)
        if val is None:
            continue
        por_valor[val]["total"] += 1
        por_valor[val]["ipas"][item["ipa_espeak"]] += 1

    regras = []
    for val, dados in por_valor.items():
        total = dados["total"]
        if total < min_ocorrencias:
            continue
        if not dados["ipas"]:
            continue
        ipa_top, freq = dados["ipas"].most_common(1)[0]
        precisao = freq / total

        if precisao < min_precisao or precisao <= baseline + 0.02:
            continue

        regras.append({
            "feature": feature_name,
            "valor": val,
            "ipa_alvo": ipa_top,
            "cobertura": total,
            "precisao": precisao,
            "ganho": precisao - baseline,
        })
    return regras


def calcular_baseline(grupo):
    contagem = Counter(item["ipa_espeak"] for item in grupo)
    total = sum(contagem.values())
    if total == 0:
        return 0.0
    _, freq = contagem.most_common(1)[0]
    return freq / total


# ---------------------------------------------------------------------------
# Descoberta de regras
# ---------------------------------------------------------------------------

def descobrir_regras_por_tipo(items, min_oc, min_prec):
    por_tipo = defaultdict(list)
    for item in items:
        tipo = item.get("tipo_mudanca") or "desconhecido"
        por_tipo[tipo].append(item)

    todas_regras = []
    for tipo, grupo in por_tipo.items():
        baseline = calcular_baseline(grupo)
        for feat in list(FEATURES_CATEGORICAS.keys()) + list(FEATURES_BOOLEANAS.keys()):
            regras = avaliar_feature(grupo, feat, min_oc, min_prec, baseline)
            for r in regras:
                r["escopo"] = "global"
                r["tipo_mudanca"] = tipo
                r["baseline"] = baseline
                todas_regras.append(r)
    return todas_regras


def descobrir_regras_por_palavra(items, min_oc, min_prec):
    por_chave = defaultdict(list)
    for item in items:
        tipo = item.get("tipo_mudanca") or "desconhecido"
        palavra = item.get("palavra")
        por_chave[(palavra, tipo)].append(item)

    todas_regras = []
    for (palavra, tipo), grupo in por_chave.items():
        if len(grupo) < min_oc:
            continue
        baseline = calcular_baseline(grupo)
        for feat in list(FEATURES_CATEGORICAS.keys()) + list(FEATURES_BOOLEANAS.keys()):
            regras = avaliar_feature(grupo, feat, min_oc, min_prec, baseline)
            for r in regras:
                r["escopo"] = "palavra"
                r["tipo_mudanca"] = tipo
                r["palavra"] = palavra
                r["baseline"] = baseline
                todas_regras.append(r)
    return todas_regras


# ---------------------------------------------------------------------------
# Ranking e deduplicação
# ---------------------------------------------------------------------------

def score_regra(r):
    return r["cobertura"] * r["ganho"] * math.log(r["precisao"] + 1)


def deduplicar(regras):
    vistas = set()
    unicas = []
    for r in regras:
        chave = (
            r.get("escopo"),
            r.get("tipo_mudanca"),
            r.get("feature"),
            str(r.get("valor")),
            r.get("ipa_alvo"),
            r.get("palavra", ""),
        )
        if chave in vistas:
            continue
        vistas.add(chave)
        unicas.append(r)
    return unicas


def consolidar_globais(regras_globais):
    agregado = defaultdict(lambda: {"cobertura": 0, "exemplos": []})
    for r in regras_globais:
        chave = (r["tipo_mudanca"], r["feature"], str(r["valor"]), r["ipa_alvo"])
        agregado[chave]["cobertura"] += r["cobertura"]
        agregado[chave]["precisao"] = r["precisao"]
        agregado[chave]["ganho"] = r["ganho"]
        agregado[chave]["baseline"] = r["baseline"]

    consolidadas = []
    for (tipo, feat, val, ipa), dados in agregado.items():
        consolidadas.append({
            "escopo": "global",
            "tipo_mudanca": tipo,
            "feature": feat,
            "valor": val,
            "ipa_alvo": ipa,
            "cobertura": dados["cobertura"],
            "precisao": dados["precisao"],
            "ganho": dados["ganho"],
            "baseline": dados["baseline"],
        })
    return consolidadas


# ---------------------------------------------------------------------------
# Análise cruzada com LLM
# ---------------------------------------------------------------------------

def cruzar_com_llm(items):
    por_gatilho = defaultdict(lambda: Counter())
    for item in items:
        g = item.get("_gatilho")
        if not g:
            continue
        for feat_nome in ["proxima_tipo", "proxima_tipo2", "anterior_tipo",
                          "posicao_bucket", "tamanho_bucket"]:
            val = item["_features"].get(feat_nome)
            if val is not None:
                por_gatilho[g][f"{feat_nome}={val}"] += 1

    resultado = {}
    for g, cont in por_gatilho.items():
        resultado[g] = cont.most_common(5)
    return resultado


# ---------------------------------------------------------------------------
# Relatório
# ---------------------------------------------------------------------------

def gerar_relatorio(regras_globais, regras_palavra, cruzamento_llm, total):
    L = []
    add = L.append

    add("=" * 78)
    add("RELATÓRIO — REGRAS CONTEXTUAIS v2")
    add("=" * 78)
    add("")
    add(f"Divergências analisadas: {total}")
    add(f"Regras globais:    {len(regras_globais)}")
    add(f"Regras por palavra:{len(regras_palavra)}")
    add("")

    # --- Regras globais ---
    add("─" * 78)
    add("1. REGRAS GLOBAIS (aplicam-se a qualquer palavra com mesmo tipo_mudanca)")
    add("─" * 78)
    add("")

    globais_ord = sorted(regras_globais, key=score_regra, reverse=True)
    for i, r in enumerate(globais_ord[:30], 1):
        add(f"[1.{i}] SE tipo={r['tipo_mudanca']} E "
            f"{r['feature']}={r['valor']} ENTÃO ipa={r['ipa_alvo']}")
        add(f"      cobertura={r['cobertura']}  precisão={r['precisao']:.2%}  "
            f"baseline={r['baseline']:.2%}  ganho={r['ganho']:+.2%}")
        add(f"      score={score_regra(r):.2f}")
        add("")

    # --- Regras por palavra ---
    add("─" * 78)
    add("2. REGRAS POR PALAVRA (específicas)")
    add("─" * 78)
    add("")

    palavra_ord = sorted(regras_palavra, key=score_regra, reverse=True)
    for i, r in enumerate(palavra_ord[:30], 1):
        add(f"[2.{i}] SE palavra='{r['palavra']}' E tipo={r['tipo_mudanca']} "
            f"E {r['feature']}={r['valor']} ENTÃO ipa={r['ipa_alvo']}")
        add(f"      cobertura={r['cobertura']}  precisão={r['precisao']:.2%}  "
            f"ganho={r['ganho']:+.2%}")
        add("")

    # --- Convergência LLM × features ---
    add("─" * 78)
    add("3. CONVERGÊNCIA LLM × FEATURES OBSERVÁVEIS")
    add("─" * 78)
    add("")
    add("  Para cada `gatilho` classificado pelo LLM, quais features")
    add("  observáveis aparecem com mais frequência?")
    add("")
    for g, cont in sorted(cruzamento_llm.items()):
        add(f"  ▸ {g}")
        for chave, n in cont:
            add(f"      {chave:<35} {n}")
        add("")

    # --- Sugestões de implementação ---
    add("─" * 78)
    add("4. SUGESTÕES DE IMPLEMENTAÇÃO EM RUST")
    add("─" * 78)
    add("")
    add("  Para cada regra global com cobertura ≥ 10 e precisão ≥ 0.9,")
    add("  sugerimos implementar diretamente em `g2p.rs`.")
    add("")

    implementaveis = [r for r in globais_ord
                      if r["cobertura"] >= 10 and r["precisao"] >= 0.9]
    for r in implementaveis[:15]:
        add(f"  // {r['tipo_mudanca']}: {r['feature']}={r['valor']}")
        add(f"  // cobertura {r['cobertura']} | precisão {r['precisao']:.0%}")
        add(f"  if tipo_mudanca == \"{r['tipo_mudanca']}\"")
        add(f"     && {r['feature']} == \"{r['valor']}\" {{")
        add(f"      ipa = \"{r['ipa_alvo']}\";")
        add(f"  }}")
        add("")

    # --- Distribuição de tipos ---
    add("─" * 78)
    add("5. TIPOS DE MUDANÇA (top 15)")
    add("─" * 78)
    add("")
    tipos = Counter()
    for r in regras_globais:
        tipos[r["tipo_mudanca"]] += r["cobertura"]
    for tipo, n in tipos.most_common(15):
        add(f"  {tipo:<30} cobertura total: {n}")

    return "\n".join(L)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--min-ocorrencias", type=int, default=3)
    ap.add_argument("--min-precisao", type=float, default=0.85)
    args = ap.parse_args()

    if not CAMINHO_DIVERGENCIAS.exists():
        imprimir(f"ERRO: {CAMINHO_DIVERGENCIAS} não encontrado")
        return 1

    imprimir(f"→ Carregando {CAMINHO_DIVERGENCIAS}")
    divergencias = json.loads(CAMINHO_DIVERGENCIAS.read_text(encoding="utf-8"))

    # Indexar gatilhos por _key
    gatilhos_por_key = {}
    if CAMINHO_GATILHOS.exists():
        imprimir(f"→ Carregando {CAMINHO_GATILHOS}")
        gatilhos = json.loads(CAMINHO_GATILHOS.read_text(encoding="utf-8"))
        for g in gatilhos:
            k = g.get("_key")
            if k:
                gatilhos_por_key[k] = g
        imprimir(f"→ {len(gatilhos_por_key)} gatilhos indexados")

    # Anotar divergências com gatilho
    itens = []
    for d in divergencias:
        k = (f"{d.get('palavra')}|{d.get('ipa_rs')}|{d.get('ipa_espeak')}|"
             f"{d.get('proxima','')}|{d.get('posicao_rel','')}|"
             f"{d.get('tem_pausa_depois', False)}")
        gat = gatilhos_por_key.get(k)
        item = dict(d)
        item["_features"] = extrair_features(d)
        # `classificacao` pode ser None, {}, ou estar ausente.
        cls = (gat or {}).get("classificacao") or {}
        item["_gatilho"] = cls.get("gatilho")
        itens.append(item)

    imprimir(f"→ {len(itens)} divergências processadas")
    com_gatilho = sum(1 for i in itens if i["_gatilho"])
    imprimir(f"→ {com_gatilho} têm classificação do LLM "
             f"({100*com_gatilho/max(len(itens),1):.0f}%)")

    if com_gatilho < len(itens) * 0.5:
        imprimir(f"  ⚠ apenas {100*com_gatilho/len(itens):.0f}% têm gatilho — "
                 f"verifique o formato do {CAMINHO_GATILHOS.name}")

    # Descobrir regras
    imprimir("→ Descobrindo regras globais...")
    regras_globais = descobrir_regras_por_tipo(
        itens, args.min_ocorrencias, args.min_precisao
    )
    imprimir(f"→ {len(regras_globais)} regras globais brutas")

    regras_globais = consolidar_globais(regras_globais)
    imprimir(f"→ {len(regras_globais)} regras globais após consolidação")

    imprimir("→ Descobrindo regras por palavra...")
    regras_palavra = descobrir_regras_por_palavra(
        itens, args.min_ocorrencias, args.min_precisao
    )
    regras_palavra = deduplicar(regras_palavra)
    imprimir(f"→ {len(regras_palavra)} regras por palavra")

    # Cruzamento LLM
    cruzamento_llm = cruzar_com_llm(itens)

    # Relatório
    relatorio = gerar_relatorio(
        regras_globais, regras_palavra, cruzamento_llm, len(itens)
    )
    imprimir("")
    imprimir(relatorio)
    CAMINHO_RELATORIO.write_text(relatorio, encoding="utf-8")
    imprimir(f"→ Salvo em {CAMINHO_RELATORIO}")

    # JSON estruturado
    CAMINHO_REGRAS.write_text(
        json.dumps({
            "globais": regras_globais,
            "por_palavra": regras_palavra,
        }, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    imprimir(f"→ Salvo em {CAMINHO_REGRAS}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())