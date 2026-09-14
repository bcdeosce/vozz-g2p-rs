#!/usr/bin/env python3
"""
gerar_lexicon_espeak_contexto.py

Gera `lexicon_espeak_contexto.json` a partir de `sentences.txt`.

Aplica o MESMO filtro do `compare_sentences.py` antes de processar:
  - descarta sentenças com tokens puramente numéricos;
  - descarta sentenças com tokens que precisam normalização;
  - descarta sentenças com siglas sem vogal em `exceptions.json`.

Para cada sentença sobrevivente:
  1. Consulta o cache `espeak_sentences.json` (compartilhado com
     `compare_sentences.py`). Se a sentença já estiver cacheada, usa
     o resultado; caso contrário, roda espeak-ng --ipa=3 e atualiza o
     cache incrementalmente.
  2. Alinha tokens de texto ↔ tokens IPA (só quando os comprimentos batem).
  3. Acumula IPA por palavra.

No final:
  4. Aplica `base_ipa` (remove sândi final) ao IPA armazenado.
  5. Compara com o IPA isolado (`lexicon_espeak.json`) também
     normalizado — se forem iguais na forma base, descarta.
  6. Emite só as palavras onde a forma base difere do isolado.

Uso:
  python3 gerar_lexicon_espeak_contexto.py sentences.txt \\
    --lexicon cache/lexicon_espeak.json \\
    --saida cache/lexicon_espeak_contexto.json \\
    --min-ocorrencias 2
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


RE_TOKEN_TEXTO = re.compile(r"[\w'-]+", re.UNICODE)
RE_PONTUACAO_BORDA = re.compile(
    r"^[;:,.!?¡¿—…\"«»“”(){}]+|[;:,.!?¡¿—…\"«»“”(){}]+$"
)
QUANTIDADE_THREADS = 32
INTERVALO_SALVAMENTO_CACHE = 500
NOME_ARQUIVO_CACHE_ESPEAK = "espeak_sentences.json"

ABREVIACOES = {
    "sr", "sra", "srta", "dr", "dra", "prof", "profa", "eng",
    "av", "r", "pç", "ed", "apto", "ap", "pág", "pag", "fig",
    "obs", "ex", "etc", "cia", "ltda", "no", "tel", "cel",
    "kg", "km", "cm", "mm", "ml", "mg", "gb", "mb", "kb", "tb",
}

VOGAIS_PORTUGUES = "aeiouáéíóúâêôãõàü"


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
    if not ipa:
        return []
    tokens = ipa.split()
    limpos = []
    for token in tokens:
        limpo = RE_PONTUACAO_BORDA.sub("", token)
        if limpo:
            limpos.append(limpo)
    return limpos


def base_ipa(ipa):
    """
    Remove a variação de sândi final, deixando a forma base.

    Regras (idempotentes):
      - `z` final → `s` (dessonorização em coda)
      - `w` final → `ʊ` (glide antes de vogal)
      - `j` final → `y` (palatalização antes de vogal)
      - `ɾ` final → `r` (tap antes de vogal)
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


def eh_puramente_numerico(palavra):
    return bool(palavra) and all(
        caractere.isdigit() and caractere.isascii()
        for caractere in palavra
    )


def eh_sigla_sem_vogal(palavra):
    minuscula = palavra.lower()
    tamanho = len(minuscula)
    if not (2 <= tamanho <= 6):
        return False
    if not minuscula.isalpha():
        return False
    return not any(caractere in VOGAIS_PORTUGUES for caractere in minuscula)


def precisa_normalizar(palavra):
    if any(not caractere.isalpha() for caractere in palavra):
        return True
    if palavra.lower() in ABREVIACOES:
        return True
    if eh_sigla_sem_vogal(palavra):
        return True
    return False


def carregar_excecoes(caminho_cache):
    caminho = Path(caminho_cache) / "exceptions.json"
    if not caminho.exists():
        return set()
    try:
        return set(json.loads(caminho.read_text(encoding="utf-8")))
    except Exception:
        return set()


def filtrar_sentencas(sentencas, excecoes):
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
# Cache do espeak-ng
# ---------------------------------------------------------------------------

def caminho_arquivo_cache_espeak(diretorio_cache):
    return Path(diretorio_cache) / NOME_ARQUIVO_CACHE_ESPEAK


def carregar_cache_espeak(caminho_arquivo):
    if not caminho_arquivo.exists():
        return {}
    try:
        dados = json.loads(caminho_arquivo.read_text(encoding="utf-8"))
        if isinstance(dados, dict):
            return dados
        return {}
    except Exception:
        return {}


def salvar_cache_espeak(caminho_arquivo, cache):
    try:
        caminho_arquivo.write_text(
            json.dumps(cache, ensure_ascii=False, indent=0),
            encoding="utf-8",
        )
    except Exception as erro:
        imprimir(f"  AVISO: falha ao salvar cache do espeak: {erro}")


def espeak_sentenca(sentenca):
    try:
        resultado = subprocess.run(
            ["espeak-ng", "-v", "pt-br", "--ipa=3", "-q", sentenca],
            capture_output=True, text=True, timeout=30,
        )
        return resultado.stdout.strip() or None
    except Exception:
        return None


def obter_ipas_espeak_com_cache(sentencas, diretorio_cache):
    caminho_cache = caminho_arquivo_cache_espeak(diretorio_cache)
    cache = carregar_cache_espeak(caminho_cache)
    if cache:
        imprimir(f"  Cache espeak carregado: {len(cache)} entradas "
                 f"({caminho_cache})")
    else:
        imprimir(f"  Cache espeak vazio (será criado em {caminho_cache})")

    sentencas_faltantes = [
        sentenca for sentenca in sentencas if sentenca not in cache
    ]
    imprimir(f"  A fonemizar: {len(sentencas_faltantes)} "
             f"(já em cache: {len(sentencas) - len(sentencas_faltantes)})")

    if not sentencas_faltantes:
        imprimir("→ espeak: todas as sentenças já estão no cache")
        return [cache.get(sentenca) for sentenca in sentencas]

    inicio = time.time()
    try:
        with ThreadPoolExecutor(max_workers=QUANTIDADE_THREADS) as executor:
            futuros = {
                executor.submit(espeak_sentenca, sentenca): sentenca
                for sentenca in sentencas_faltantes
            }
            concluidos = 0
            for futuro in as_completed(futuros):
                sentenca = futuros[futuro]
                try:
                    cache[sentenca] = futuro.result()
                except Exception:
                    cache[sentenca] = None
                concluidos += 1
                if concluidos % INTERVALO_SALVAMENTO_CACHE == 0:
                    salvar_cache_espeak(caminho_cache, cache)
                    decorrido = time.time() - inicio
                    imprimir(
                        f"  [espeak] {concluidos}/{len(sentencas_faltantes)} "
                        f"({decorrido:.0f}s) — cache salvo"
                    )
                elif concluidos % 100 == 0:
                    decorrido = time.time() - inicio
                    imprimir(
                        f"  [espeak] {concluidos}/{len(sentencas_faltantes)} "
                        f"({decorrido:.0f}s)"
                    )
    except KeyboardInterrupt:
        imprimir("  Interrompido! Salvando cache parcial...")
        salvar_cache_espeak(caminho_cache, cache)
        raise

    salvar_cache_espeak(caminho_cache, cache)
    tempo_total = time.time() - inicio
    imprimir(
        f"→ espeak: {tempo_total:.2f}s "
        f"({len(sentencas_faltantes)} novas, {len(cache)} no cache)"
    )
    return [cache.get(sentenca) for sentenca in sentencas]


# ---------------------------------------------------------------------------
# Alinhamento e geração do léxico
# ---------------------------------------------------------------------------

def alinhar(sentenca, ipa_espeak):
    palavras = tokenizar_texto(sentenca)
    tokens_ipa = tokenizar_ipa(normalizar_ipa(ipa_espeak))

    if not palavras or not tokens_ipa:
        return None

    quantidade_palavras = len(palavras)
    quantidade_tokens_ipa = len(tokens_ipa)

    if quantidade_palavras == quantidade_tokens_ipa:
        return list(zip(palavras, tokens_ipa))

    if abs(quantidade_palavras - quantidade_tokens_ipa) > max(
        quantidade_palavras, quantidade_tokens_ipa
    ) // 2:
        return None

    comprimentos = [len(palavra) for palavra in palavras]
    total_comprimentos = sum(comprimentos)
    resultado = []

    if quantidade_tokens_ipa >= quantidade_palavras:
        indice_token = 0
        for posicao, palavra in enumerate(palavras):
            fatia = max(
                1,
                round(comprimentos[posicao] / total_comprimentos
                      * quantidade_tokens_ipa),
            )
            fim = min(
                indice_token + fatia,
                quantidade_tokens_ipa - (quantidade_palavras - posicao - 1),
            )
            if fim <= indice_token:
                fim = indice_token + 1
            ipa_parte = "".join(tokens_ipa[indice_token:fim])
            resultado.append((palavra, ipa_parte))
            indice_token = fim
    else:
        indice_palavra = 0
        for posicao_token, token in enumerate(tokens_ipa):
            tokens_restantes = quantidade_tokens_ipa - posicao_token - 1
            palavras_disponiveis = quantidade_palavras - indice_palavra
            if posicao_token == quantidade_tokens_ipa - 1:
                grupo = palavras[indice_palavra:]
            else:
                fatia = max(
                    1,
                    round(palavras_disponiveis / (tokens_restantes + 1)),
                )
                grupo = palavras[indice_palavra:indice_palavra + fatia]
            if len(grupo) == 1:
                resultado.append((grupo[0], token))
            indice_palavra += len(grupo)

    return resultado


def carregar_lexicon_isolado(caminho):
    if not caminho or not Path(caminho).exists():
        return {}
    with open(caminho, encoding="utf-8") as arquivo:
        return json.load(arquivo)


def gerar(
    sentencas,
    diretorio_cache,
    lexicon_isolado,
    chave_composta=False,
    min_ocorrencias=2,
    min_fracao=0.9,
):
    imprimir("→ Obtendo IPA do espeak-ng (com cache compartilhado)...")
    ipas_espeak = obter_ipas_espeak_com_cache(sentencas, diretorio_cache)

    acumulado = defaultdict(Counter)
    total_por_chave = Counter()
    estruturais = 0

    imprimir("→ Alinhando e acumulando IPA por palavra...")
    for indice, sentenca in enumerate(sentencas):
        ipa = ipas_espeak[indice] if indice < len(ipas_espeak) else None
        if ipa is None:
            continue

        alinhamento = alinhar(sentenca, ipa)
        if alinhamento is None:
            estruturais += 1
            continue

        for posicao, (palavra, ipa_palavra) in enumerate(alinhamento):
            base = palavra.lower().strip("'-")
            if not base:
                continue
            if chave_composta:
                proxima = ""
                if posicao + 1 < len(alinhamento):
                    proxima = (
                        alinhamento[posicao + 1][0].lower().strip("'-")
                    )
                chave = f"{base}|{proxima}"
            else:
                chave = base
            acumulado[chave][ipa_palavra] += 1
            total_por_chave[chave] += 1

    imprimir(
        f"→ {estruturais} sentenças não alinhadas "
        f"(comprimentos diferentes)"
    )
    imprimir(f"→ {len(acumulado)} chaves distintas acumuladas")

    resultado = {}
    descartadas_iguais = 0
    descartadas_espalhadas = 0
    descartadas_sem_mudanca_base = 0

    for chave, contagem in acumulado.items():
        total = total_por_chave[chave]
        if total < min_ocorrencias:
            continue

        ipa_mais_comum, frequencia = contagem.most_common(1)[0]
        fracao = frequencia / total

        if fracao < min_fracao:
            descartadas_espalhadas += 1
            continue

        # Compara com o isolado **na forma base**. Se a forma base do
        # contexto bate com a forma base do isolado, a diferença era
        # só sândi — o runtime vai aplicar de novo com a próxima
        # palavra correta.
        if not chave_composta:
            ipa_isolado = lexicon_isolado.get(chave)
            if ipa_isolado:
                base_isolado = base_ipa(normalizar_ipa(ipa_isolado))
                base_contexto = base_ipa(ipa_mais_comum)
                if base_isolado == base_contexto:
                    descartadas_iguais += 1
                    continue
                # Diferença real que NÃO é só sândi de final.
                # Armazena a forma base.
                resultado[chave] = base_contexto
                descartadas_sem_mudanca_base += 1
                continue

        # Sem isolado para comparar: armazena a forma base.
        resultado[chave] = base_ipa(ipa_mais_comum)

    imprimir(
        f"→ {descartadas_iguais} chaves iguais ao isolado (na forma base)"
    )
    imprimir(
        f"→ {descartadas_espalhadas} chaves espalhadas "
        f"(< {min_fracao}, removidas)"
    )
    imprimir(f"→ {len(resultado)} entradas finais")

    return resultado


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("sentencas")
    parser.add_argument("--lexicon", default="cache/lexicon_espeak.json")
    parser.add_argument(
        "--saida", default="cache/lexicon_espeak_contexto.json"
    )
    parser.add_argument(
        "--cache", default="cache",
        help="Diretório do cache (exceptions.json e espeak_sentences.json)",
    )
    parser.add_argument("--min-ocorrencias", type=int, default=2)
    parser.add_argument("--min-fracao", type=float, default=0.9)
    parser.add_argument("--chave-composta", action="store_true")
    parser.add_argument("--limite", type=int, default=0)
    argumentos = parser.parse_args()

    caminho = Path(argumentos.sentencas).resolve()
    if not caminho.is_file():
        imprimir(f"ERRO: {caminho} não encontrado")
        sys.exit(1)

    diretorio_cache = Path(argumentos.cache).resolve()
    diretorio_cache.mkdir(parents=True, exist_ok=True)

    sentencas_brutas = [
        linha.strip()
        for linha in caminho.read_text(encoding="utf-8").split("\n")
        if linha.strip()
    ]
    imprimir(f"→ {len(sentencas_brutas)} sentenças lidas")

    excecoes = carregar_excecoes(diretorio_cache)
    imprimir(f"→ {len(excecoes)} siglas nas exceções")
    sentencas, descartadas = filtrar_sentencas(sentencas_brutas, excecoes)
    imprimir(
        f"→ {len(sentencas)} sentenças após filtro "
        f"({descartadas} descartadas)"
    )

    if argumentos.limite > 0:
        sentencas = sentencas[:argumentos.limite]
        imprimir(f"→ Limitado a {len(sentencas)} sentenças")

    if not sentencas:
        imprimir("ERRO: nenhuma sentença restou após o filtro.")
        sys.exit(1)

    lexicon_isolado = carregar_lexicon_isolado(argumentos.lexicon)
    imprimir(f"→ {len(lexicon_isolado)} entradas no lexicon isolado")

    resultado = gerar(
        sentencas,
        diretorio_cache,
        lexicon_isolado,
        chave_composta=argumentos.chave_composta,
        min_ocorrencias=argumentos.min_ocorrencias,
        min_fracao=argumentos.min_fracao,
    )

    saida = Path(argumentos.saida)
    saida.parent.mkdir(parents=True, exist_ok=True)
    with open(saida, "w", encoding="utf-8") as arquivo:
        json.dump(resultado, arquivo, ensure_ascii=False, indent=2)
    imprimir(f"→ Salvo em {saida}")

    imprimir("")
    imprimir("AMOSTRA (top 20):")
    for chave, ipa in sorted(resultado.items())[:20]:
        isolado = lexicon_isolado.get(chave, "?")
        imprimir(f"  {chave:<25} contexto={ipa:<20} isolado={isolado}")


if __name__ == "__main__":
    main()