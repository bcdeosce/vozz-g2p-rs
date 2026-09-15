```markdown
# Scripts do vozz-g2p-rs

Referência completa dos utilitários Python do projeto, organizados por
categoria. Todos exigem:

- Rust compilado (`cargo build --release --bin phonemizer-worker`)
- `espeak-ng` instalado (`apt install espeak-ng`)
- Python 3.10+
- Node.js 18+ (apenas para os scripts que comparam com o Vozz JS)
- `pip install aiohttp tiktoken datasets` (alguns scripts específicos)

---

## Índice

- [1. Benchmarks](#1-benchmarks)
  - [compare_vozz.py](#compare_vozzpy)
  - [compare_sentences.py](#compare_sentencespy)
  - [compare_sentences_bifonia.py](#compare_sentences_bifoniapy)
- [2. Geração de léxicos](#2-geração-de-léxicos)
  - [gerar_cache_diacritizado.py](#gerar_cache_diacritizadopy)
  - [gerar_lexicon_homografos.py](#gerar_lexicon_homografospy)
  - [gerar_lexicon_espeak_contexto.py](#gerar_lexicon_espeak_contextopy)
  - [normalizar_lexicon_espeak.py](#normalizar_lexicon_espeakpy)
- [3. Validação](#3-validação)
  - [validar_homografos_diacritizados.py](#validar_homografos_diacritizadospy)
  - [verificar_gabarito.py](#verificar_gabaritopy)
- [4. Descoberta de padrões](#4-descoberta-de-padrões)
  - [mapear_regra_palavra.py](#mapear_regra_palavrapy)
  - [extrair_candidatos_lexicon.py](#extrair_candidatos_lexiconpy)
  - [extrair_padroes_classificador.py](#extrair_padroes_classificadorpy)
  - [extrair_regras_contextuais_v2.py](#extrair_regras_contextuais_v2py)
  - [testar_generalizacao_sandhi.py](#testar_generalizacao_sandhipy)
  - [testar_regras_por_palavra.py](#testar_regras_por_palavrapy)
  - [analise_divergencias_contexto.py](#analise_divergencias_contextopy)
- [5. Aquisição de corpus](#5-aquisição-de-corpus)
  - [baixar_leipzig.py](#baixar_leipzigpy)

---

## 1. Benchmarks

### `compare_vozz.py`

**O que faz:** Compara três fonetizadores em **palavras isoladas**:
Vozz JS, vozz-rs e espeak-ng. Gera relatório + léxico IPA do espeak.

**Saídas:**
- `cache/relatorio.txt` — relatório com categorias e exemplos
- `cache/lexico_espeak.json` — léxico `palavra → IPA` (gabarito espeak)
- `cache/exceptions.json` — siglas sem vogal descartadas
- `cache/tempos.json` — estatísticas de tempo
- `cache/js.json`, `cache/espeak.json`, `cache/rs.json` — saídas brutas

**Flags:**

| Flag | Efeito |
|---|---|
| `corpus.txt` | (posicional) caminho do corpus |
| `--skip-js` | Pula o Vozz JS (usa cache existente) |
| `--force-all` | Ignora todos os caches, regenera tudo |
| `--force-espeak` | Só força regeração do espeak |
| `--no-test` | Pula `cargo test` antes de rodar |
| `--threads N` | Threads paralelas para espeak (default: 32) |

**Uso:**

```bash
# Rodada completa
python3 compare_vozz.py corpus.txt

# Iterar rápido (só RS vs espeak)
python3 compare_vozz.py corpus.txt --skip-js

# Forçar regeração
python3 compare_vozz.py corpus.txt --force-all

# Mais threads
python3 compare_vozz.py corpus.txt --threads 64
```

**Tempo típico:** 73k palavras → ~2,5s (js) + ~1,2s (rs) + ~950s (espeak primeira rodada).

---

### `compare_sentences.py`

**O que faz:** Compara três fonetizadores em **sentenças** de um corpus
genérico (médico). Mesma estrutura do `compare_vozz.py`, mas preserva
contexto de sentença.

**Saídas:**
- `cache/sentencas_resultados.json` — cache com IPA por sentença
- `cache/espeak_sentences.json` — cache espeak isolado
- Relatório completo no stdout

**Flags:**

| Flag | Efeito |
|---|---|
| `sentences.txt` | (posicional) caminho do corpus |
| `--limite N` | Limita o número de sentenças (0 = todas) |
| `--diretorio-cache PATH` | Diretório de cache customizado |
| `--max-exemplos N` | Exemplos por categoria no relatório (default: 5) |

**Uso:**

```bash
# Rodada completa
python3 compare_sentences.py sentences.txt

# Teste rápido
python3 compare_sentences.py sentences.txt --limite 1000
```

**Tempo típico:** 5.980 sentenças → ~1s (js) + ~1,3s (rs) + ~115s (espeak).

---

### `compare_sentences_bifonia.py`

**O que faz:** Versão especializada para o corpus Bifonia. Além da
comparação padrão, inclui:

1. **Cache em `espeak_sentences_bifonia.json`** — poupa ~20 min do espeak em re-execuções.
2. **Análise de homógrafos** — acurácia específica por palavra do Bifonia.
3. **Distribuição de erros por sentença** — quantas frases têm 0, 1, 2, 3, 4, 5+ erros, com %.

**Saídas:**
- `cache/espeak_sentences_bifonia.json` — cache do espeak
- `cache/relatorio_bifonia.txt` — relatório completo

**Flags:**

| Flag | Efeito |
|---|---|
| `sentences_bifonia.txt` | (posicional) corpus |
| `--limite N` | Limita sentenças |
| `--force-espeak` | Regenera o cache do espeak |
| `--cache PATH` | Diretório de cache |
| `--cache-espeak PATH` | Caminho do cache específico (default: `cache/espeak_sentences_bifonia.json`) |

**Uso:**

```bash
# Rodada completa (primeira vez: ~20 min de espeak)
python3 compare_sentences_bifonia.py sentences_bifonia.txt

# Re-execução (usa cache, ~30s)
python3 compare_sentences_bifonia.py sentences_bifonia.txt

# Forçar regeração
python3 compare_sentences_bifonia.py sentences_bifonia.txt --force-espeak

# Usar outro corpus
python3 compare_sentences_bifonia.py sentences_val.txt \
  --cache-espeak cache/espeak_sentences_val.json
```

**Tempo típico:** 102k sentenças → ~9,3s (js) + ~18s (rs) + ~1.225s (espeak primeira rodada).

---

## 2. Geração de léxicos

### `gerar_cache_diacritizado.py`

**O que faz:** Roda `espeak-ng` em todas as sentenças **diacritizadas**
do dataset Bifonia (`diacritized_sentence`). Filtra sentenças que
precisam de normalização (números, abreviações, siglas sem vogal).
Gera o cache usado pelo `gerar_lexicon_homografos.py`.

**Por que usar a versão diacritizada:** o diacrítico força a leitura
correta do homógrafo. Sem ele, o espeak escolhe aleatoriamente.

**Saída:**
- `cache/espeak_sentences_bifonia_diac.json` — cache `{sentencas, espeak}`

**Flags:**

| Flag | Efeito |
|---|---|
| `bifonia.jsonl` | (posicional) dataset com `diacritized_sentence` |
| `--saida PATH` | Caminho do cache (default: `cache/espeak_sentences_bifonia_diac.json`) |
| `--cache PATH` | Diretório de `exceptions.json` |
| `--force` | Ignora cache existente, regenera |

**Uso:**

```bash
# Primeira vez (~15-25 min)
python3 gerar_cache_diacritizado.py bifonia_pt_homographs_no_ipa.jsonl

# Regenerar
python3 gerar_cache_diacritizado.py bifonia_pt_homographs_no_ipa.jsonl --force
```

**Saída típica:**
```
→ 40156 sentenças únicas no dataset
→ 32418 após filtro (7738 descartadas)
→ 0 faltam no cache
→ Cache salvo em cache/espeak_sentences_bifonia_diac.json
```

---

### `gerar_lexicon_homografos.py`

**O que faz:** Gera `lexicon_homografos.json` — mapa
`(palavra, sentido) → IPA`. Usa o `diacritized_sentence` como entrada
para o espeak, garantindo que cada sentido receba o IPA correto.

**Saída:**
- `cache/lexicon_homografos.json` — `{ palavra: { sentido: { ipa, confianca, ocorrencias } } }`

**Flags:**

| Flag | Efeito |
|---|---|
| `--dataset PATH` | JSONL do Bifonia (obrigatório) |
| `--regras PATH` | `homograph_rules_v2.json` (obrigatório) |
| `--saida PATH` | Caminho de saída (default: `cache/lexicon_homografos.json`) |
| `--cache PATH` | Diretório de cache |
| `--cache-espeak PATH` | Caminho do cache diacritizado |
| `--min-ocorrencias N` | Mínimo de exemplos por sentido (default: 1) |
| `--usar-cru` | Usa `sentence` em vez de `diacritized_sentence` |

**Uso:**

```bash
# Rodar (usa cache diacritizado existente)
python3 gerar_lexicon_homografos.py \
  --dataset bifonia_pt_homographs_no_ipa.jsonl \
  --regras cache/homograph_rules_v2.json \
  --saida cache/lexicon_homografos.json \
  --cache cache \
  --min-ocorrencias 1
```

**Saída típica:**
```
→ 131 palavras-alvo | 131 no léxico
FILTRAGEM DO JSONL
  → 124339 usam diacritized_sentence
  → 21627 sentenças fora do cache
  → 102712 exemplos válidos
→ 101946 exemplos com IPA extraído
→ 131 palavras no lexicon final
```

---

### `gerar_lexicon_espeak_contexto.py`

**O que faz:** Gera `lexico_espeak_contexto.json` — mapa
`palavra → IPA contextual`. Roda espeak nas sentenças do corpus, alinha
por token e agrega por palavra. Aplica `base_ipa` (remove sândi final)
e só salva quando a forma base difere do léxico isolado.

**Saída:**
- `cache/lexico_espeak_contexto.json` — `{ palavra: ipa }`
- Atualiza `cache/espeak_sentences.json` incrementalmente

**Flags:**

| Flag | Efeito |
|---|---|
| `sentences.txt` | (posicional) corpus |
| `--lexicon PATH` | Léxico isolado para comparar (default: `cache/lexico_espeak.json`) |
| `--saida PATH` | Saída (default: `cache/lexico_espeak_contexto.json`) |
| `--cache PATH` | Diretório do cache |
| `--min-ocorrencias N` | Mínimo de exemplos por palavra (default: 2) |
| `--min-fracao F` | Fração mínima do IPA mais comum (0.0-1.0, default: 0.9) |
| `--chave-composta` | Chave = `palavra\|próxima_palavra` |
| `--limite N` | Limita sentenças |

**Uso:**

```bash
python3 gerar_lexicon_espeak_contexto.py sentences.txt \
  --lexicon cache/lexico_espeak.json \
  --saida cache/lexico_espeak_contexto.json \
  --cache cache \
  --min-ocorrencias 2 \
  --min-fracao 0.9
```

**Saída típica:**
```
→ 7463 sentenças lidas
→ 0 entradas no lexicon isolado
→ 11059 chaves distintas acumuladas
→ 2063 chaves espalhadas (< 0.9, removidas)
→ 3431 entradas finais
```

---

### `normalizar_lexicon_espeak.py`

**O que faz:** Aplica `base_ipa` (remove sândi final: `z→s`, `w→ʊ`,
`j→y`, `ɾ→r`) em todas as entradas do léxico do espeak. Útil para
corrigir léxicos gerados com notação antiga.

**Cuidado:** não aplique no `lexico_espeak.json` se ele for usado em
modo isolado — o `palavra_para_ipa` já aplica `limpar`. Use só para
diagnóstico ou para o léxico contextual.

**Saída:**
- Arquivo novo com sufixo `_v7.json` (ou `--saida`)

**Flags:**

| Flag | Efeito |
|---|---|
| `lexicon.json` | (posicional) entrada |
| `--saida PATH` | Arquivo de saída |
| `--dry-run` | Mostra o que mudaria sem salvar |

**Uso:**

```bash
# Ver o escopo da mudança
python3 normalizar_lexicon_espeak.py cache/lexico_espeak.json --dry-run

# Aplicar
python3 normalizar_lexicon_espeak.py cache/lexico_espeak.json \
  --saida cache/lexico_espeak_v7.json
```

---

## 3. Validação

### `validar_homografos_diacritizados.py`

**O que faz:** Valida o pipeline de homógrafos contra o **gabarito
diacritizado**. Compara:
- `RS(sentence)` — pipeline processando a sentença crua
- `espeak(diacritized_sentence)` — espeak forçado à leitura correta

**Categoriza os erros** (o mais útil):
- **A. Classificador** — escolheu o sentido errado
- **B. Léxico** — léxico com IPA errado (base)
- **C. Tratamento** — sândi/pós-processamento divergiu
- **D. Sem regra** — palavra não tem regra

Canonicaliza `i/y/j` e `ʊ/w` para não contar como erro notacional.

**Saída:**
- `cache/validacao_homografos.json` — `{stats, por_categoria}`

**Flags:**

| Flag | Efeito |
|---|---|
| `--dataset PATH` | JSONL do Bifonia |
| `--cache-espeak PATH` | Cache espeak diacritizado |
| `--regras PATH` | `homograph_rules_v2.json` |
| `--lexicon PATH` | `lexicon_homografos.json` |
| `--worker PATH` | Caminho do binário (default: `target/release/phonemizer-worker`) |
| `--saida PATH` | JSON de saída |
| `--max-por-palavra N` | Limita exemplos por palavra (default: 50) |

**Uso:**

```bash
python3 validar_homografos_diacritizados.py \
  --dataset bifonia_pt_homographs_no_ipa.jsonl \
  --cache-espeak cache/espeak_sentences_bifonia_diac.json \
  --regras cache/homograph_rules_v2.json \
  --lexicon cache/lexicon_homografos.json
```

**Saída típica:**
```
Total de exemplos:                        6472
OK (não-erro / notacional):               6407
Erros reais:                              65

A. Classificador escolheu sentido errado: 61
B. Léxico com IPA errado (base):          0
C. Tratamento (sândi/pós-processamento):  4
D. Sem regra:                             0
```

---

### `verificar_gabarito.py`

**O que faz:** Para cada erro do tipo "classificador" (identificado pelo
validador acima), roda espeak na **sentença crua** e verifica de que lado
ele fica:
- Se concorda com o **dataset** → o classificador errou
- Se concorda com o **pipeline** → o espeak também errou

Responde: "quem está certo nos casos de divergência?"

**Saída:**
- `cache/verificacao_gabarito.json` — `{veredito, detalhes}`

**Flags:**

| Flag | Efeito |
|---|---|
| `--dataset PATH` | JSONL do Bifonia |
| `--cache-diario PATH` | Cache diacritizado |
| `--validacao PATH` | `validacao_homografos.json` |
| `--saida PATH` | JSON de saída |

**Uso:**

```bash
python3 verificar_gabarito.py \
  --dataset bifonia_pt_homographs_no_ipa.jsonl \
  --cache-diario cache/espeak_sentences_bifonia_diac.json \
  --validacao cache/validacao_homografos.json
```

**Saída típica:**
```
espeak_concorda_com_dataset     38  ( 62.3%)
espeak_concorda_com_pipeline    23  ( 37.7%)
```

---

## 4. Descoberta de padrões

### `mapear_regra_palavra.py`

**O que faz:** Para uma palavra, roda espeak + RS em **6 contextos
controlados** e mostra lado a lado onde divergem:

| Contexto | Template |
|---|---|
| antes de vogal | `Eu vejo {p} água` |
| antes de sonora | `Eu vejo {p} bola` |
| antes de surda | `Eu vejo {p} coisa` |
| fim de sentença | `Eu vejo {p}` |
| início de sentença | `{p} está aqui` |
| isolado | `{p}` |

Útil para descobrir se uma palavra tem regra contextual.

**Flags:**

| Flag | Efeito |
|---|---|
| `--palavra P` | (obrigatório) palavra a analisar |
| `--worker PATH` | Caminho do binário |

**Uso:**

```bash
python3 mapear_regra_palavra.py --palavra onde
```

**Saída típica:**
```
Contexto               espeak                 RS                     Match
------------------------------------------------------------------------
antes de vogal         ˌoŋdʒj                 ˌoŋdʒj                 ✓
antes de sonora        ˌoŋdʒy                 ˌoŋdʒj                 ✗
antes de surda         ˌoŋdʒy                 ˌoŋdʒj                 ✗
fim de sentença        ˌoŋdʒy                 ˌoŋdʒj                 ✗
início de sentença     ˌoŋdʒj                 ˌoŋdʒj                 ✓
isolado                ˈoŋdʒy                 ˈoŋdʒy                 ✓
```

---

### `extrair_candidatos_lexicon.py`

**O que faz:** Lê `sentencas_resultados.json` e agrupa divergências por
par `(RS, espeak)`. Prioriza por frequência. Útil para popular
manualmente o `lexicon_contexto.json`.

**Saída:** stdout (relatório textual)

**Flags:**

| Flag | Efeito |
|---|---|
| `--min-ocorrencias N` | Filtra pares com N+ ocorrências (default: 3) |
| `--top N` | Top N pares (default: 50) |

**Uso:**

```bash
python3 extrair_candidatos_lexicon.py --min-ocorrencias 5 --top 50
```

**Saída típica:**
```
RS                        espeak                     total  palavras
--------------------------------------------------------------------
ky                        ke                           319  que(319)
vosˈe                     vosˌe                        262  você(262)
seɪŋ                      sˈeɪŋ                         98  sem(98)
```

---

### `extrair_padroes_classificador.py`

**O que faz:** Lê `validacao_homografos.json` (saída do validador) e
analisa os erros do tipo "classificador". Agrupa por palavra, lista os
contextos mais frequentes e **sugere expressões fixas** para o
`homografos.rs`.

**Saída:** stdout (relatório textual)

**Uso:**

```bash
python3 extrair_padroes_classificador.py
```

**Saída típica:**
```
▸ molho  (14 erros)
    esperado=bundle          predito=sauce           (9)
    esperado=sauce           predito=bundle          (5)
    prev_words: [('o', 8), ('um', 4), ('recomendo', 1), ('cada', 1)]
    next_words: [('de', 14)]

SUGESTÕES DE EXPRESSÕES FIXAS
    ("molho", Some("o"), None, "bundle", "bundle"),
    ...
```

---

### `extrair_regras_contextuais_v2.py`

**O que faz:** Analisa `divergencias_contexto_v2.json` (features ricas
por divergência) e `deepinfra_gatilhos_v2.json` (classificação do LLM).
Extrai regras candidatas: features que separam IPAs do espeak.

**Saída:**
- `cache/relatorio_regras_v2.txt`
- `cache/regras_contextuais_v2.json`

**Flags:**

| Flag | Efeito |
|---|---|
| `--min-ocorrencias N` | Mínimo (default: 3) |
| `--min-precisao F` | Precisão mínima (default: 0.85) |

**Uso:**

```bash
python3 extrair_regras_contextuais_v2.py --min-ocorrencias 3
```

---

### `testar_generalizacao_sandhi.py`

**O que faz:** Para cada tipo de mudança fonológica (`sandhi_z_s`,
`notacao_y_j`, `notacao_ʊ_w`, etc.), mede se o gatilho **generaliza
entre palavras** ou é assinatura lexical de uma só.

**Saída:**
- `cache/relatorio_generalizacao_sandhi.txt`

**Flags:**

| Flag | Efeito |
|---|---|
| `--tipo TIPO` | Analisa só este tipo_mudanca |
| `--cache PATH` | Caminho do `divergencias_contexto_v2.json` |

**Uso:**

```bash
python3 testar_generalizacao_sandhi.py
python3 testar_generalizacao_sandhi.py --tipo sandhi_z_s
```

**Saída típica:**
```
TIPO: sandhi_z_s (530 ocorrências)
  Palavras distintas: 318
  Palavra dominante: horas (7.9%)

  proxima_tipo=vogal
    total=319  palavras=193  top=horas(13%)
    IPA top: ˈɔɾæs (13%)
    consistência interna: 100%
    → GENERALIZA
```

---

### `testar_regras_por_palavra.py`

**O que faz:** Para cada palavra com ≥N ocorrências em
`divergencias_contexto_v2.json`, classifica em:
- **Lexical** — IPA fixo, sem variação contextual
- **Ambíguo** — ambos variam
- **Regra faltando** — espeak varia, RS não
- **Sândi inverso** — RS varia, espeak não

**Saída:**
- `cache/relatorio_regras_por_palavra.txt`

**Flags:**

| Flag | Efeito |
|---|---|
| `--palavra P` | Analisa só esta |
| `--min-ocorrencias N` | Mínimo de ocorrências (default: 10) |
| `--top N` | Top N palavras (default: 30) |
| `--verboso` | Mostra todas as features testadas |

**Uso:**

```bash
python3 testar_regras_por_palavra.py --min-ocorrencias 20 --top 50
python3 testar_regras_por_palavra.py --palavra de --verboso
```

---

### `analise_divergencias_contexto.py`

**O que faz:** Pipeline de 3 fases para análise com LLM:

1. **Extrair** — gera `divergencias_contexto.json` a partir de
   `sentencas_resultados.json`, com features ricas.
2. **Classificar** — envia cada divergência ao DeepInfra, pedindo
   classificação do gatilho fonológico.
3. **Relatar** — agrega por palavra e por tipo de mudança.

**Saída:**
- `cache/divergencias_contexto.json`
- `cache/deepinfra_gatilhos.json`
- `cache/relatorio_gatilhos.txt`

**Flags:**

| Flag | Efeito |
|---|---|
| `--fase {extrair,classificar,relatar,tudo}` | Qual fase rodar (default: tudo) |
| `--limite N` | Limita divergências enviadas ao LLM |

**Requer:** `DEEPINFRA_TOKEN_1` ... `DEEPINFRA_TOKEN_10` no ambiente.

**Uso:**

```bash
# Pipeline completo com 500 itens
python3 analise_divergencias_contexto.py --fase tudo --limite 500

# Só a extração
python3 analise_divergencias_contexto.py --fase extrair

# Só a agregação (após classificação)
python3 analise_divergencias_contexto.py --fase relatar
```

**Custo:** ~$0,03 para 500 itens em DeepInfra flex.

---

## 5. Aquisição de corpus

### `baixar_leipzig.py`

**O que faz:** Baixa e processa o corpus Leipzig
(`por-br_newscrawl_2011_100K`). Extrai o `.txt` do `.tar.gz`, limpa
numeração, normaliza espaços, filtra por tamanho e deduplica.

**Saída:**
- `sentences_val.txt` (ou `--saida`)

**Flags:**

| Flag | Efeito |
|---|---|
| `--url URL` | URL customizada |
| `--arquivo-local PATH` | Usa arquivo local em vez de baixar |
| `--saida PATH` | Saída (default: `sentences_val.txt`) |
| `--min-palavras N` | Descarta sentenças com menos de N palavras (default: 3) |
| `--max-palavras N` | Descarta com mais de N (default: 60) |
| `--timeout N` | Timeout do download (default: 60) |

**Uso:**

```bash
# Download automático
python3 baixar_leipzig.py

# Usar arquivo local já baixado
python3 baixar_leipzig.py --arquivo-local por-br_newscrawl_2011_100K-sentences.txt

# URL customizada
python3 baixar_leipzig.py --url https://meu-servidor/corpus.txt
```

**Saída típica:**
```
→ Baixando https://downloads.wortschatz-leipzig.de/corpora/...
  12.3 MB baixados
→ 100000 sentenças brutas (0 linhas descartadas)
→ 59682 sentenças após filtro de tamanho
→ 59682 sentenças únicas
→ Salvo em sentences_val.txt
```

---

## Fluxo de trabalho recomendado

### Setup inicial (uma vez)

```bash
# 1. Baixar Leipzig
python3 baixar_leipzig.py

# 2. Gerar cache do espeak para Leipzig
python3 compare_sentences_bifonia.py sentences_val.txt \
  --cache-espeak cache/espeak_sentences_val.json
```

### Ciclo de iteração

```bash
# 1. Gerar cache diacritizado (só quando o dataset muda)
python3 gerar_cache_diacritizado.py bifonia_pt_homographs_no_ipa.jsonl

# 2. Regenerar léxico de homógrafos
python3 gerar_lexicon_homografos.py \
  --dataset bifonia_pt_homographs_no_ipa.jsonl \
  --regras cache/homograph_rules_v2.json \
  --saida cache/lexicon_homografos.json \
  --cache cache

# 3. Validar
python3 validar_homografos_diacritizados.py \
  --dataset bifonia_pt_homographs_no_ipa.jsonl \
  --cache-espeak cache/espeak_sentences_bifonia_diac.json \
  --regras cache/homograph_rules_v2.json \
  --lexicon cache/lexicon_homografos.json

# 4. Verificar gabarito nos casos de erro
python3 verificar_gabarito.py \
  --dataset bifonia_pt_homographs_no_ipa.jsonl \
  --cache-diario cache/espeak_sentences_bifonia_diac.json \
  --validacao cache/validacao_homografos.json

# 5. Extrair padrões para expressões fixas
python3 extrair_padroes_classificador.py

# 6. (Editar homografos.rs e/ou bigrama.rs manualmente)

# 7. Recompilar e revalidar
cargo build --release --bin phonemizer-worker
python3 validar_homografos_diacritizados.py [...]
```

### Diagnóstico pontual

```bash
# Por que uma palavra está errada?
python3 mapear_regra_palavra.py --palavra sede

# Quais candidatos ao lexicon_contexto?
python3 extrair_candidatos_lexicon.py --min-ocorrencias 10
```

---

## Resumo por frequência de uso

| Script | Uso típico |
|---|---|
| `validar_homografos_diacritizados.py` | Validação principal (rodar sempre) |
| `gerar_lexicon_homografos.py` | Regenerar léxico após mudança no dataset |
| `compare_sentences_bifonia.py` | Benchmark geral (rodar menos) |
| `mapear_regra_palavra.py` | Investigar palavra específica |
| `extrair_padroes_classificador.py` | Após validação, para sugerir regras |
| `verificar_gabarito.py` | Auditar erros de classificador |
| `baixar_leipzig.py` | Aquisição de corpus externo |
| `analise_divergencias_contexto.py` | Descoberta com LLM (custo $) |
| `compare_vozz.py` | Benchmark de palavras isoladas |
| `testar_*` | Uso pontual para diagnóstico |
```
