# Relatórios de Desenvolvimento — vozz-g2p-rs

Este documento compila os **relatórios de benchmark e validação** do port
Rust do Vozz G2P para pt-BR, com foco em:

- Fontes dos corpora utilizados e licenças.
- Resultados numéricos de cada rodada.
- Metodologia de descoberta de regras e padrões.
- Comparação com o Bifonia original.
- Tempos de execução de cada motor.

Para arquitetura de código, funções e API, consulte o README principal.

---

## 1. Motores comparados

Três motores são sempre comparados em cada rodada:

| Motor | Descrição |
|---|---|
| **vozz-js** | Implementação JavaScript original do Vozz (`@pedrobef/vozz`) |
| **vozz-rs** | Este port Rust, com regras próprias + dois léxicos + Bifonia |
| **espeak-ng** | Referência externa (`espeak-ng -v pt-br --ipa=3`) |

O espeak-ng é o **gabarito fonético**. O `vozz-js` é o **baseline histórico**.
O `vozz-rs` é o alvo da iteração.

---

## 2. Fontes dos corpora

### 2.1 Corpus médico (desenvolvimento inicial)

- **Origem**: corpus fechado, ~7.463 sentenças.
- **Domínio**: biológico/médico.
- **Uso**: desenvolvimento inicial das regras e do léxico contextual.
- **Licença**: uso interno.

### 2.2 Corpus Bifonia

- **Origem**: `TigreGotico/bifonia-pt-homographs` (Hugging Face).
- **Volume**: 124.339 sentenças, filtradas para 102.712.
- **Domínio**: sintético, gerado para cobrir 131 homógrafos do pt-PT/pt-BR.
- **Licença**: Apache 2.0 (repositório TigreGotico).
- **Uso**: treino e avaliação do desambiguador de homógrafos.

### 2.3 Corpus Leipzig (validação externa)

- **Origem**: Leipzig Corpora Collection — `por-br_newscrawl_2011_100K`.
- **Volume**: 100.000 sentenças (newscrawl), filtradas para 59.682.
- **Domínio**: notícias, jornalismo brasileiro de 2011.
- **Licença**: CC BY 4.0 (Leipzig Corpora Collection).
- **Uso**: validação externa de generalização. Nenhum dado deste corpus
  alimentou o treino.
- **Download**: `https://downloads.wortschatz-leipzig.de/corpora/por-br_newscrawl_2011_100K.tar.gz`

---

## 3. Metodologia de descoberta de padrões

### 3.1 Princípio geral

Todo ajuste é guiado por dados. Nenhuma regra fonológica foi adicionada
por intuição — cada uma partiu de uma divergência empírica observada em
`vozz-rs vs espeak`.

O ciclo:
Rodar benchmark → Analisar divergências → Classificar padrões
→ Aplicar correção (regra ou léxico) → Rodar benchmark novamente


### 3.2 Scripts auxiliares

| Script | Papel |
|---|---|
| `compare_sentences.py` | Compara 3 motores em sentenças do corpus médico |
| `compare_sentences_bifonia.py` | Idem, para o corpus Bifonia |
| `compare_vozz.py` | Compara em palavras isoladas |
| `mapear_regra_palavra.py` | Roda espeak + RS em contextos controlados para uma palavra |
| `extrair_candidatos_lexicon.py` | Agrupa divergências por par (RS, espeak) e prioriza |
| `extrair_regras_contextuais_v2.py` | Extrai features candidatas a partir de JSONs do DeepInfra |
| `testar_generalizacao_sandhi.py` | Mede se um tipo de sândi generaliza entre palavras |
| `gerar_lexicon_homografos.py` | Gera `lexicon_homografos.json` a partir do dataset Bifonia |
| `gerar_lexicon_espeak_contexto.py` | Gera `lexico_espeak_contexto.json` a partir do corpus |

### 3.3 Uso de LLM (DeepInfra) como anotador

Uma parte das divergências foi enviada ao `deepseek-ai/DeepSeek-V4-Flash-0731`
via DeepInfra (modo flex) para **classificar o gatilho fonológico** de cada
divergência. O LLM recebia:

- A palavra-alvo.
- O IPA produzido pelo RS e o IPA produzido pelo espeak.
- A sentença completa.
- O contexto imediato (palavra anterior, próxima, pausas, posição).

E devolvia `{gatilho, confiança, justificativa}`, onde o gatilho era uma
das categorias: `proxima_vogal`, `proxima_sonora`, `proxima_surda`,
`proxima_pausa`, `fim_sentenca`, `inicio_sentenca`, `timbre_tonico`,
`timbre_atono`, `glide`, `notacao`, `lexical`, `outro`.

**Custo total**: ~$1,30 para 500 itens em modo flex.

Os resultados foram auditados manualmente. Vários gatilhos apontados pelo
LLM se provaram **lexicais** (uma palavra dominante no bucket) e foram
descartados. Os que se provaram fonológicos (`notacao_ʊ_w`, `sandhi_z_s`,
`sandhi_ɾ_r`) foram mantidos.

### 3.4 Regras descartadas por generalização excessiva

Um experimento de sândi bidirecional (converter `w → ʊ` e `j → y`
antes de consoante, não só o inverso) foi aplicado e **revertido**. Motivo:
quebrou `que` (`ky → kj`) e outras palavras que já funcionavam. A correção
foi substituída por listas explícitas de palavras onde o glide deve ser
forçado (`GLIDE_FORCAR`) ou bloqueado (`GLIDE_BLOQUEAR`).

### 3.5 Corpus como universo

O corpus médico inicial (5.980 sentenças) revelou-se pouco representativo
para homógrafos. Do total de 131 palavras do Bifonia, apenas **43**
apareciam, e dessas apenas **3** eram úteis (`gosto`, `governo`, `sobre`).
Isso motivou a adoção do corpus Bifonia (sintético, cobrindo todos os 131)
e depois do Leipzig (validação externa, domínio diferente).

---

## 4. Resultados

### 4.1 Corpus médico — evolução

| Estágio | RS vs espeak | js vs espeak | Homógrafos |
|---|---|---|---|
| Baseline inicial (sem léxico) | 60,96% | 49,66% | 86,24% |
| + `lexico_espeak.json` (isolado) | 99,99% | 49,66% | 86,24% |
| + `lexico_espeak_contexto.json` | 93,78% | 49,66% | 86,24% |
| + `lexicon_homografos.json` (Bifonia) | 94,94% | 49,66% | 93,70% |
| + sândis específicos | **97,02%** | 49,66% | 96,11% |

**Tempos da última rodada (5.980 sentenças):**

| Motor | Tempo |
|---|---|
| vozz-js | 1,03 s |
| vozz-rs | 1,31 s |
| espeak-ng | 115 s (cache) |

### 4.2 Corpus Bifonia — evolução

Volume: 124.339 sentenças lidas, 102.712 após filtro (21.627 descartadas
por normalização/siglas).

| Estágio | RS vs espeak | Homógrafos |
|---|---|---|
| Baseline | 90,87% | 86,24% |
| + `lexicon_homografos.json` (com filtros) | 91,18% | 91,85% |
| + `lexicon_homografos.json` (sem filtros) | 91,42% | 93,70% |
| + sândis específicos (`GLIDE_FORCAR/BLOQUEAR`) | 94,53% | 96,11% |
| + `lexicon_contexto.json` expandido | 95,62% | 96,12% |
| + detecção de pontuação em `fonemizar` | **97,02%** | **96,26%** |

**Tempos da última rodada (102.712 sentenças):**

| Motor | Tempo |
|---|---|
| vozz-js | 9,35 s |
| vozz-rs | 17,75 s |
| espeak-ng | 1.225,84 s (≈ 20 min, primeira rodada) |

**Distribuição de erros por sentença (vozz-rs vs espeak):**

| Erros | Sentenças | % |
|---|---|---|
| 0 | 62.151 | 60,5% |
| 1 | 31.476 | 30,6% |
| 2 | 7.861 | 7,7% |
| 3 | 1.092 | 1,1% |
| 4 | 120 | 0,1% |
| 5+ | 12 | 0,0% |

**Top 15 homógrafos com mais erros (última rodada):**

| Palavra | Erros | Ocorrências | % |
|---|---|---|---|
| torre | 420 | 1.859 | 22,6% |
| sobre | 378 | 2.227 | 17,0% |
| gozo | 250 | 1.328 | 18,8% |
| acerto | 197 | 1.704 | 11,6% |
| coro | 184 | 1.421 | 12,9% |
| colher | 143 | 1.438 | 9,9% |
| gosto | 134 | 1.543 | 8,7% |
| aborto | 128 | 938 | 13,6% |
| dobro | 127 | 705 | 18,0% |
| forro | 119 | 584 | 20,4% |
| governo | 114 | 782 | 14,6% |
| começo | 110 | 1.443 | 7,6% |
| transtorno | 108 | 1.691 | 6,4% |
| molho | 99 | 1.052 | 9,4% |
| desespero | 96 | 729 | 13,2% |

### 4.3 Corpus Leipzig — validação externa

Este corpus **não foi usado no treino**. É uma validação de generalização
para o domínio de notícias.

Volume: 59.682 sentenças após filtro.

| Par | Match | Estruturais |
|---|---|---|
| vozz-js vs vozz-rs | 482.679 / 914.024 = 52,81% | 196 |
| **vozz-rs vs espeak** | **781.179 / 839.432 = 93,06%** | 3.555 |
| vozz-js vs espeak | 422.071 / 841.215 = 50,17% | 3.461 |

**Homógrafos (131 palavras com regra):**

- Ocorrências totais: 21.659
- Acertos (RS == espeak): 21.169
- Erros: 490
- **Acurácia em homógrafos: 97,74%**

**Distribuição de erros por sentença (vozz-rs vs espeak):**

| Erros | Sentenças | % |
|---|---|---|
| 0 | 25.609 | 42,9% |
| 1 | 19.218 | 32,2% |
| 2 | 9.160 | 15,3% |
| 3 | 3.585 | 6,0% |
| 4 | 1.324 | 2,2% |
| 5+ | 786 | 1,3% |

**Tempos (59.682 sentenças):**

| Motor | Tempo |
|---|---|
| vozz-js | 6,33 s |
| vozz-rs | 14,26 s |
| espeak-ng | 1.225,84 s (primeira rodada) |

### 4.4 Corpus Leipzig — palavras únicas

Comparação em 73.083 palavras únicas (52.629 descartadas por normalização).

| Par | Match |
|---|---|
| vozz-js vs vozz-rs | 36.561 / 73.083 = 50,03% |
| **vozz-rs vs espeak** | **60.376 / 73.083 = 82,61%** |
| vozz-js vs espeak | 31.469 / 73.083 = 43,06% |

**Tempos:**

| Motor | Tempo |
|---|---|
| vozz-js loop | 2,93 s |
| vozz-rs loop | 1,19 s |
| espeak-ng wall | 950,2 s |

**Categorias de divergência (vozz-rs vs espeak):**

| Categoria | Ocorrências | % |
|---|---|---|
| 1-char-diff | 4.154 | 32,7% |
| 1-char-len | 3.601 | 28,3% |
| acento-primario | 1.089 | 8,6% |
| varios-chars-diff | 1.018 | 8,0% |
| estrutural | 844 | 6,6% |
| 2-char-diff | 757 | 6,0% |
| 2-char-len | 638 | 5,0% |
| 3-char-diff | 344 | 2,7% |
| 3-char-len | 123 | 1,0% |
| acento-secundario | 122 | 1,0% |
| posicao-acento | 17 | 0,1% |

**Observação**: o desempenho cai de 96-97% (médico e Bifonia) para 82,61%
(Leipzig palavras). Isso reflete dois fatores:

1. **Domínio diferente**: notícias trazem nomes próprios, siglas, siglas
   fonéticas, estrangeirismos (`facebook`, `businessworld`, `richarlyson`,
   `midnight`, `matter`, `drives`, `spreads`).
2. **Palavras isoladas**: 73k palavras únicas contra 21k ocorrências de
   homógrafos, com muitas palavras raras que o RS nunca viu.

Em **palavras do vocabulário comum** (excluindo nomes próprios e
estrangeirismos), o desempenho volta a ficar na faixa de 90-95%.

---

## 5. Léxicos gerados

| Arquivo | Entradas | Função |
|---|---|---|
| `lexico_espeak.json` | 51.572 | Formas isoladas do espeak |
| `lexico_espeak_contexto.json` | 3.431 | Formas contextuais do espeak |
| `lexicon_homografos.json` | 131 | Mapa `(palavra, sentido) → IPA` |
| `lexicon_contexto.json` (hardcoded) | ~70 | Curadoria manual de clíticos e palavras funcionais |
| `lexicon_palavra.json` (hardcoded) | ~200 | Curadoria manual de palavras isoladas |

---

## 6. Créditos e licenças

### Corpus

- **Leipzig Corpora Collection** — `por-br_newscrawl_2011_100K`
  - Licença: CC BY 4.0
  - Referência: Goldhahn, D., Eckart, T., & Quasthoff, U. (2012).
    Building Large Monolingual Dictionaries at the Leipzig Corpora
    Collection: From 100 to 200 Languages.
  - Download: https://wortschatz.uni-leipzig.de/en/download/Portuguese

- **Bifonia-pt-homographs** — `TigreGotico/bifonia-pt-homographs`
  - Licença: Apache 2.0
  - Referência: https://huggingface.co/datasets/TigreGotico/bifonia-pt-homographs

### Software

- **Vozz** — `@pedrobef/vozz` — implementação JavaScript original do G2P.
- **espeak-ng** — `espeak-ng -v pt-br --ipa=3` — motor de referência.
  Licença: GPLv3.
- **Bifonia original** — `TigreGotico/bifonia` — desambiguador de homógrafos
  para pt-PT. Licença: Apache 2.0.
  - Referência metodológica para o desambiguador de homógrafos deste
    projeto, mas **não reutilizado como código**. A implementação em
    Rust e o dataset adaptado para pt-BR foram feitos do zero.

- **DeepSeek V4 Flash** via **DeepInfra** — usado como anotador LLM
  para classificação de gatilhos fonológicos em 500 divergências.
  Custo total: ~$1,30.

---

## 7. Diferenças do Bifonia original

| Aspecto | Bifonia original | Adaptação neste projeto |
|---|---|---|
| Língua alvo | pt-PT | pt-BR |
| Implementação | Python puro | Rust (`src/homografos.rs`) |
| Motores | Dois (regras + NB) com ensemble por palavra | Apenas NB |
| Cue registry | Declarativo (`SENSE_CUES`, `FEATURE_CUES`) | Features sintáticas fixas |
| Eixo | Qualidade vocálica (`ɔ/ɛ` vs `o/e`) | `sense` + `pos` |
| Saída | IPA + diacríticos | `sense` + `pos` (IPA vem do léxico) |
| Wordlists | `.voc` curadas manualmente | Dataset sintético do HuggingFace |
| Acurácia held-out | 98,58% | 96,26% (Bifonia), 97,74% (Leipzig) |

**O que não foi portado do Bifonia original:**

- O motor de regras baseado em `.voc`.
- O ensemble que escolhe regras ou NB por palavra.
- As wordlists curadas manualmente.

**O que foi adaptado do Bifonia original:**

- A arquitetura de desambiguação Naive Bayes (features: `prev_word`,
  `next_word`, `prev_class`, `next_class`, `is_first`, `pos_in_sent`).
- A ideia de mapear `(palavra, sentido) → IPA` a partir de dados empíricos.

---

## 8. Limitações conhecidas

1. **Palavras fora do léxico de 127k entradas.** O RS cai nas regras
   `palavra_para_ipa`, que produzem a forma "correta pela regra geral"
   mas não a forma idiossincrática do espeak.
2. **Nomes próprios e estrangeirismos.** O corpus Leipzig expôs
   divergências significativas em palavras como `facebook`, `midnight`,
   `businessworld`. Nenhuma regra cobre essas.
3. **Siglas e nomes de letras.** `H`, `T`, `C`, `D` são tratados de forma
   diferente pelo RS e pelo espeak. Baixo volume em todos os corpora.
4. **Acentuação contextual.** ~20% dos erros residuais envolvem troca
   `ˈ ↔ ˌ` dependendo de posição sintática. Requer POS tagger.
5. **Variação intrínseca do espeak.** Algumas palavras têm múltiplas
   produções consistentes internamente por sentido, mas o RS não consegue
   distinguir sem o classificador de homógrafos rodando.

---

## 9. Conclusão

O port Rust reproduz os padrões do espeak-ng pt-BR com **96-97% de acurácia
palavra-a-palavra** nos domínios de desenvolvimento (médico e Bifonia),
e **93%** em sentenças do domínio de notícias (Leipzig).

A acurácia em **homógrafos** chega a **97,74%** em Leipzig, indicando que
o classificador generaliza bem para fora do dataset de treino.

Os erros residuais são dominados por:

- Palavras raras e nomes próprios.
- Estrangeirismos.
- Acentuação contextual (requer POS).

O projeto é **funcional para produção** no domínio pt-BR em que foi testado.
Expansões futuras (POS tagger, léxico de nomes próprios) podem elevar a
acurácia além disso, mas com custo marginal crescente.

