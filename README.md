```markdown
# vozz-g2p-rs

Conversor **grafema→fonema (G2P) para português do Brasil**, escrito em Rust puro, com foco em fidelidade à convenção IPA do `espeak-ng pt-br` — a mesma convenção com que modelos neurais de TTS (Piper, Coqui, VITS) são treinados.

Este projeto é um **fork/porte do [Vozz](https://github.com/Pedro21062014/vozz)**, uma biblioteca JavaScript de G2P para pt-BR. O objetivo é ter a mesma qualidade de fonetização em um binário nativo, sem dependência de runtime JS e com performance muito superior.

Esta versão traz **três mudanças estruturais** em relação ao porte inicial:

1. **Desambiguação de homógrafos via [BCDE-tagger](https://github.com/bcdeosce/BCDE-tagger)** — substitui o classificador Naive Bayes + bigramas anterior.
2. **Regras de trema do Acordo Ortográfico de 1990** — resolve `qu`/`gu` antes de `e`/`i` corretamente.
3. **Pipeline de síntese para o [Piper](https://github.com/rhasspy/piper)** — saída no alfabeto do modelo, chunking inteligente, controle de pausa e **6 presets de emoção**.

---

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org)
[![Build](https://img.shields.io/badge/build-passing-brightgreen.svg)]()
[![Fidelity espeak-ng](https://img.shields.io/badge/espeak--ng%20pt--br-97%25-success.svg)]()
[![Homógrafos](https://img.shields.io/badge/hom%C3%B3grafos-97.7%25-success.svg)]()
[![Emoções](https://img.shields.io/badge/emo%C3%A7%C3%B5es-6-blueviolet.svg)]()

## Índice

- [O que mudou nesta versão](#o-que-mudou-nesta-versão)
- [Motivação](#motivação)
- [Diferenças em relação ao Vozz original](#diferenças-em-relação-ao-vozz-original)
- [Instalação](#instalação)
- [Arquivos de dados e léxicos](#arquivos-de-dados-e-léxicos)
- [Uso como biblioteca Rust](#uso-como-biblioteca-rust)
- [Uso como worker (CLI)](#uso-como-worker-cli)
- [Pipeline Piper + emoções](#pipeline-piper--emoções)
- [Calibração por voz](#calibração-por-voz)
- [Cliente Python (referência)](#cliente-python-referência)
- [Comparador Python](#comparador-python)
- [API pública](#api-pública)
- [Regras fonológicas implementadas](#regras-fonológicas-implementadas)
- [Testes](#testes)
- [FAQ](#faq)
- [Licença](#licença)
- [Créditos](#créditos)

---

## O que mudou nesta versão

| Área | Antes | Agora |
|------|-------|-------|
| Desambiguação de homógrafos | Naive Bayes + bigramas (`homografos.rs`, `bigrama.rs`) | [BCDE-tagger](https://github.com/bcdeosce/BCDE-tagger) via lib |
| Léxico de homógrafos | `(palavra, sentido) → IPA` dentro do desambiguador | `lexicon_homografos.rs` (mapa puro) |
| Trema (`qu`/`gu` antes de `e`/`i`) | Não tratado | `trema.rs` com reintrodução hipotética + lista curada |
| Saída para Piper | Nenhuma | `piper.rs` (tokens) + `piper_pipeline.rs` (chunks + pausas) |
| Emoções | Nenhuma | 6 presets: `neutro`, `ansioso`, `cansado`, `triste`, `empolgado`, `calmo` |
| Chunking inteligente | Nenhum | Funde sentenças curtas (≤ 200 chars); fragmenta só acima disso |
| Pausas por pontuação | Fixas | Base calibrada × fator de emoção |
| Actions do worker | `version`, `set_lexicon`, `process`, `process_batch` | + `process_piper`, `process_piper_chunks` |

**Removidos:** `src/homografos.rs`, `src/bigrama.rs`, `data/bigramas.json`, `data/homograph_rules_v2.json`.

**Adicionados:** `src/tagger.rs`, `src/trema.rs`, `src/piper.rs`, `src/piper_pipeline.rs`, `src/lexicon_homografos.rs`, `data/regras_trema.json`.

---

## Motivação

O [Vozz](https://github.com/Pedro21062014/vozz) é uma biblioteca JS excelente para fonetização pt-BR, mas tem algumas limitações que motivaram este porte:

- **Performance**: o G2P em JS roda a ~21 µs/palavra. Em Rust, o mesmo pipeline roda a ~1,5 µs/palavra (14x mais rápido).
- **Integração**: um binário nativo é mais fácil de integrar em pipelines Python, C++, Go, ou em servidores de TTS de alta performance.
- **Precisão**: durante o porte, várias regras foram revisitadas com base em comparação direta contra o `espeak-ng pt-br`, corrigindo bugs do original.
- **Auditoria**: as regras estão em Rust fortemente tipado, com testes unitários cobrindo cada decisão fonológica.
- **Homógrafos**: o classificador NB original era bom, mas o BCDE-tagger eleva a acurácia em casos que o NB não pegava (mesmo POS com sentidos distintos).
- **Piper**: modelos Piper são treinados com a convenção do espeak-ng, mas exigem **segmentação por sentença** e **pausas controladas** para soar natural. Este projeto centraliza toda essa lógica no Rust.

---

## Diferenças em relação ao Vozz original

Este não é um porte mecânico. Durante o desenvolvimento, foram identificados e corrigidos vários bugs e divergências do Vozz original em relação ao `espeak-ng pt-br`:

| Regra | Vozz original | vozz-g2p-rs |
|-------|---------------|-------------|
| `x` intervocálico | Sempre `/ʃ/` | `/ʃ/` por padrão; `/ks/` em radicais eruditos (`taxi-`, `fix-`, `hidrox-`, `oxid-`, etc.) |
| Prefixo `ex-` + vogal | `/ʃ/` | `/z/` (`exame`, `exemplo`, `exato`) |
| Acento secundário | Só na 1ª sílaba se tônica na 3ª+ | Múltiplas sílabas (0, 2, 4, ...), alinhado ao espeak |
| `-am` final de verbo | `ɐ̃ŋ` | `ɐ̃ʊ̃` (`falam`, `cantam`) |
| Ditongos nasais gráficos | `ã` + `o` era 2 sílabas | `-ão`, `-ãe`, `-õe` formam um núcleo |
| `-ou` final | `oʊ` | `ow` (`cantou`, `falou`) |
| `-eu` final | `ew` | `eʊ` (`vendeu`, `meu`) |
| `-au` final | `aw` | `aʊ` (`pau`, `causa`) |
| `-ei`/`-ai` finais | Nem sempre oxítonas | Sempre oxítonas (`cantei`, `abafai`) |
| `-is`/`-us` final de verbo | Paroxítonas | Oxítonas (`medis`, `impus`) |
| `l` em coda | Sempre `w` | `w` (final/átona); `ʊ` (tônica: `alto`, `palma`) |
| `nh` nasalização | Só `a` tônico | `a` tônico **ou pré-tônico**, e `u` |
| `c`/`g`/`d`/`t` + `i` + vogal | Ditongo crescente | Hiato (`cianeto`, `girasol`, `diádico`) |
| `-ídeo` final | `ideʊ` | `idʒjʊ` (`radionuclídeo`) |
| Prefixo `sobre-` | Acento secundário na 1ª | Átono em palavras de 4+ sílabas |
| **Trema (novo)** | Não tratado | `cinquenta` → `sĩˈkwẽtɐ` (antes: `sĩˈkẽtɐ`) |
| **Homógrafo `sede`** | Não desambiguado | `sˈedʒi` (seat) vs `sˈɛdʒi` (thirst) via BCDE |

O `vozz-g2p-rs` atinge **~90% de fidelidade** contra o `espeak-ng pt-br` no corpus de 336k palavras, contra ~50% do Vozz original.

---

## Instalação

### Requisitos

- Rust 1.70+ (`rustup` recomendado)
- (Opcional) `espeak-ng` — só para rodar o comparador Python
- (Opcional) Node.js 18+ — só para rodar o comparador Python contra o Vozz JS
- (Opcional) Python 3.10+ — só para os scripts de geração e para o cliente Piper

### Como dependência (Cargo.toml)

```toml
[dependencies]
vozz-g2p-rs = { git = "https://github.com/bcdeosce/vozz-g2p-rs" }
```

O `bcde-tagger` entra como dependência transitiva:

```toml
bcde-tagger = { git = "https://github.com/bcdeosce/BCDE-tagger" }
```

### Build do binário

```bash
git clone https://github.com/bcdeosce/vozz-g2p-rs.git
git clone https://github.com/bcdeosce/BCDE-tagger

cd BCDE-tagger   && cargo build --release
cd ../vozz-g2p-rs && cargo build --release
```

O binário fica em `./target/release/phonemizer-worker`.

O worker espera a variável `BCDE_TAGGER_DATA` apontando para o diretório de dados do tagger:

```bash
export BCDE_TAGGER_DATA=/caminho/para/BCDE-tagger/data
```

---

## Arquivos de dados e léxicos

O pipeline usa **7 arquivos de dados** em cascata:

| Arquivo | Entradas | Escopo | Fonte |
|---------|----------|--------|-------|
| `data/lexicon_palavra.json` | ~200 | Palavras isoladas + clíticos | Curadoria manual |
| `data/lexicon_contexto.json` | ~70 | Clíticos + palavras funcionais em contexto | Curadoria manual |
| `data/lexicon_homografos.json` | ~131 | `(palavra, sentido) → IPA` | Curadoria manual |
| `data/regras_trema.json` | 605 + 40 radicais | Lista de palavras com trema + radicais produtivos | [IME-USP](https://www.ime.usp.br/~pf/dicios/br-com-trema-latin1.txt) |
| `cache/lexico_espeak.json` | ~51k | Formas isoladas do espeak | `compare_vozz.py` |
| `cache/lexico_espeak_contexto.json` | ~3.4k | Formas contextuais do espeak | `gerar_lexicon_espeak_contexto.py` |
| `BCDE-tagger/data/*.json` | — | Tagger POS + diacríticos + resolvedor | Repo oficial do BCDE-tagger |

### Como regenerar cada um

**`lexico_espeak.json`** (espeak isolado, ~51k entradas):

```bash
python3 compare_vozz.py corpus.txt
```

**`lexico_espeak_contexto.json`** (espeak contextual):

```bash
python3 gerar_lexicon_espeak_contexto.py
```

**`regras_trema.json`** — gerado a partir do arquivo de palavras com trema do IME-USP + lista de radicais produtivos. Créditos na seção final.

**`lexicon_homografos.json`** — mantido por curadoria manual, chaveado por `(palavra, sentido)`.

---

## Uso como biblioteca Rust

### Fonetização simples

```rust
use vozz_g2p_rs::fonemizar;
use vozz_g2p_rs::g2p::OpcoesFonemizar;

let ipa = fonemizar("Olá, mundo!", &OpcoesFonemizar::default());
println!("{}", ipa);
// olˈa, mˈũŋdʊ!
```

### Fonetização com tagger + homógrafos

```rust
use vozz_g2p_rs::{fonemizar, normalizar, OpcoesNormalizar};
use vozz_g2p_rs::g2p::OpcoesFonemizar;
use vozz_g2p_rs::tagger::{self, Tagger};
use vozz_g2p_rs::lexicon_homografos::LexiconHomografos;
use std::path::Path;

fn main() -> Result<(), String> {
    // Carrega o tagger uma vez e mantém vivo.
    let tagger = tagger::carregar_tagger("BCDE-tagger/data")?;

    // Carrega o léxico (palavra, sentido) → IPA.
    let hom = LexiconHomografos::from_path(
        Path::new("data/lexicon_homografos.json"))?;

    let opcoes = OpcoesFonemizar {
        normalizar: true,
        lexico: None,
        lexico_contexto: None,
        homografos: Some(&hom),
        tagger: Some(&tagger),
    };

    let texto = "Ele tem sede de justiça e sede em Campinas.";
    let normalizado = normalizar(texto, OpcoesNormalizar::default());
    println!("{}", fonemizar(&normalizado, &opcoes));
    // Duas leituras distintas de "sede" desambiguadas pelo tagger.

    Ok(())
}
```

### Fonetização com léxico customizado

```rust
use vozz_g2p_rs::fonemizar;
use vozz_g2p_rs::g2p::OpcoesFonemizar;
use std::collections::HashMap;

let mut lexico = HashMap::new();
lexico.insert("zendesk".to_string(), "zẽdˈɛski".to_string());
lexico.insert("kubernetes".to_string(), "kubeɾnˈetʃis".to_string());

let opcoes = OpcoesFonemizar {
    normalizar: true,
    lexico: Some(&lexico),
    ..Default::default()
};

let ipa = fonemizar("Rodamos em Kubernetes e Zendesk.", &opcoes);
println!("{}", ipa);
```

### Apenas uma palavra

```rust
use vozz_g2p_rs::palavra_para_ipa;

println!("{}", palavra_para_ipa("exemplo", None));
// ˌezˈeɪmplʊ
```

### Trema (`cinquenta`, `tranquilo`, `linguiça`)

A regra do trema é aplicada automaticamente em `palavra_para_ipa` e `fonemizar`:

```rust
use vozz_g2p_rs::palavra_para_ipa;

println!("{}", palavra_para_ipa("cinquenta", None));
// sĩˈkwẽtɐ    ← trema hipotético detectado
println!("{}", palavra_para_ipa("quente", None));
// ˈkẽtʃi       ← sem trema, u mudo
```

### Silabificação

```rust
use vozz_g2p_rs::silabificar;

for silaba in silabificar("banana") {
    println!("onset={:?} nucleo={:?} coda={:?} tonica={}",
        silaba.onset, silaba.nucleo, silaba.coda, silaba.tonica);
}
```

### Normalização

```rust
use vozz_g2p_rs::{normalizar, OpcoesNormalizar};

let texto = normalizar("R$ 1.234,56 em 07/09/2025.", OpcoesNormalizar::default());
println!("{}", texto);
// mil duzentos e trinta e quatro reais e cinquenta e seis centavos
// em sete de setembro de dois mil e vinte e cinco.
```

### Divisão em sentenças

```rust
use vozz_g2p_rs::dividir_em_sentencas;

let sentencas = dividir_em_sentencas("Olá! Como vai? Bem, obrigado.");
for s in sentencas {
    println!("- {}", s);
}
```

### Pipeline Piper: IPA → chunks + pausas

```rust
use vozz_g2p_rs::piper::ipa_para_piper;
use vozz_g2p_rs::piper_pipeline::preparar_chunks;

let ipa = "ʊ xˈatʊ xoˈew. ˈmais aˈinda.";
let tokens = ipa_para_piper(ipa);
let ipa_piper: String = tokens.concat();

let chunks = preparar_chunks(&ipa_piper, "triste");
for ch in &chunks {
    println!("ls={} pausa_apos={}ms",
             ch.length_scale, ch.pausa_apos_ms);
    for fr in &ch.fragments {
        println!("  {}  ({}ms)", fr.ipa, fr.pausa_ms);
    }
}
```

---

## Uso como worker (CLI)

O `phonemizer-worker` é um processo persistente: lê JSON de `stdin`, escreve JSON em `stdout`, uma linha por requisição. Mantenha o processo vivo num servidor para evitar recarregar o BCDE-tagger (~500 ms por chamada).

### Iniciar

```bash
export BCDE_TAGGER_DATA=/caminho/para/BCDE-tagger/data
./target/release/phonemizer-worker
```

### Ações suportadas

#### `version`

```json
{"action":"version"}
```

```json
{
  "build": "2024-11-piper-chunks-v2",
  "features": ["set_lexicon","process","process_piper",
               "process_piper_chunks","process_batch","tagger","trema"]
}
```

#### `set_lexicon`

Recarrega os léxicos em runtime. Hot-reload sem reiniciar o worker.

```json
{
  "action": "set_lexicon",
  "base_path": "data/lexico_espeak.json",
  "global_path": "data/global.json",
  "context_path": "data/lexico_espeak_contexto.json",
  "voice_paths": {"idoso": "data/voz_idoso.json"},
  "voice_paths_contexto": {"idoso": "data/voz_idoso_ctx.json"}
}
```

#### `process` — IPA puro por sentença

Compatível com projetos que consomem IPA cru.

```json
{"action":"process","text":"O rato roeu a roupa."}
```

```json
{
  "sentences": [
    {"text":"O rato roeu a roupa.","phonemes":"ʊ xˈatʊ xoˈew a xˈowpɐ."}
  ]
}
```

#### `process_piper` — tokens individuais

Devolve tokens no alfabeto do Piper (`c→k`, `æ→ɐ`, `y→ɪ`, `ow→oʊ`, `aj→aɪ`).

```json
{"action":"process_piper","text":"O rato roeu a roupa."}
```

```json
{
  "sentences": [
    {
      "text":"O rato roeu a roupa.",
      "phonemes":["ʊ"," ","x","ˈ","a","t","ʊ"," ","x","o","ˈ","e","w",
                  " ","a"," ","x","ˈ","o","ʊ","p","ɐ","."]
    }
  ]
}
```

#### `process_piper_chunks` — **recomendado para síntese**

Faz chunking inteligente + cálculo de pausas por emoção. Devolve tudo pronto para o cliente sintetizar.

```json
{
  "action":"process_piper_chunks",
  "text":"O Sr. Silva tem sede de justiça e sede em Campinas; depois de cinquenta anos, ele colheu os frutos tranquilos do seu trabalho.",
  "emocao":"triste"
}
```

```json
{
  "ipa_piper": "ʊ sˈeɲoɾ sˈiwvɐ tẽj sˈedʒi dʒi ʒustˈisɐ i sˈedʒi ẽj kɐ̃pˈinɐs; depˈojs dʒi sĩkwˈẽtɐ ˈɐnʊs, ˈeli kolˈew ʊs fɾˈutʊs tɾɐ̃kwˈilʊs dʊ sˈew tɾabˈaʎʊ.",
  "emocao": "triste",
  "chunks": [
    {
      "fragments": [
        {"ipa":"ʊ sˈeɲoɾ sˈiwvɐ tẽj sˈedʒi dʒi ʒustˈisɐ i sˈedʒi ẽj kɐ̃pˈinɐs","pausa_ms":434},
        {"ipa":"depˈojs dʒi sĩkwˈẽtɐ ˈɐnʊs","pausa_ms":434},
        {"ipa":"ˈeli kolˈew ʊs fɾˈutʊs tɾɐ̃kwˈilʊs dʊ sˈew tɾabˈaʎʊ","pausa_ms":0}
      ],
      "length_scale": 1.08,
      "pausa_apos_ms": 0
    }
  ]
}
```

Campos:
- `fragments[].pausa_ms` — silêncio a inserir **depois** daquele fragmento.
- `chunks[].length_scale` — passa direto para `SynthesisConfig` do Piper.
- `chunks[].pausa_apos_ms` — silêncio a inserir entre chunks.

#### `process_batch` — dicionário `palavra → IPA`

```json
{"action":"process_batch","words":["sede","cinquenta","colheu"]}
```

```json
{
  "phonemes": {
    "sede":"sˈedʒi",
    "cinquenta":"sĩkwˈẽtɐ",
    "colheu":"kolˈew"
  },
  "timing_ms": 3,
  "total": 3
}
```

---

## Pipeline Piper + emoções

### Visão geral

```
texto bruto
  ↓
normalizar.rs          ← datas, números, moedas, siglas
  ↓
splitter.rs            ← divide em sentenças
  ↓
g2p.rs                 ← por palavra:
  ├─ tagger.rs           ↳ BCDE devolve `sense`
  ├─ lexicon_homografos.rs ↳ (palavra, sense) → IPA
  ├─ lexicon_contexto.rs / lexicon_palavra.rs
  ├─ trema.rs            ↳ `qu`/`gu` antes de e/i
  └─ regras de silabificação, tonicidade, nasalização, sândi
  ↓
IPA puro
  ↓
piper.rs               ← IPA → alfabeto do Piper
  ↓
piper_pipeline.rs      ← chunking + pausas × emoção
  ↓
JSON para o cliente
  ↓
PiperVoice.synthesize_wav("[[ IPA ]]")   ← no cliente Python
```

### Emoções

Cada emoção ajusta dois parâmetros globais:

| Emoção | `length_scale` | `fator_pausa` | Efeito |
|--------|:-:|:-:|--------|
| `neutro`    | 1.00 | 1.00 | Fala padrão |
| `ansioso`   | 0.86 | 0.40 | Mais rápido, pausas curtas |
| `cansado`   | 1.25 | 1.80 | Mais devagar, pausas longas |
| `triste`    | 1.08 | 1.55 | Leve desaceleração, pausas longas |
| `empolgado` | 0.92 | 0.55 | Rápido, pausas curtas |
| `calmo`     | 1.05 | 1.20 | Leve desaceleração, pausas médias |

- `length_scale` < 1 acelera, > 1 desacelera. Passado direto para `SynthesisConfig` do Piper.
- `fator_pausa` multiplica as durações base das pausas de pontuação.

### Pausas base (modo neutro)

| Pontuação | Duração |
|-----------|:-:|
| `,` | 180 ms |
| `;` | 280 ms |
| `:` | 350 ms |
| `.` | 500 ms |
| `!` | 450 ms |
| `?` | 450 ms |
| `…` | 750 ms |

Ajustáveis em `src/piper_pipeline.rs` (`PAUSAS_BASE`, `EMOCOES`).

### Como passar a emoção

Na chamada do worker:

```python
req({"action": "process_piper_chunks",
     "text":   "Ele se foi. Sem se despedir.",
     "emocao": "triste"})
```

Em Rust puro:

```rust
use vozz_g2p_rs::piper_pipeline::preparar_chunks;

let chunks = preparar_chunks(&ipa_piper, "triste");
```

### Chunking inteligente

O módulo `piper_pipeline.rs` implementa:

1. **Se o IPA total ≤ 200 chars** → 1 chunk único.
   Evita sintetizar `"Ele se foi."` / `"Sem se despedir."` / `"Fiquei aqui."` / `"Sozinho."` separadamente, o que soa robótico.
2. **Se > 200 chars** → corta nos `.`/`!`/`?`/`…` e agrupa pedaços até chegar em ~200 chars por chunk.
3. Dentro de cada chunk, cada fragmento carrega sua própria pausa (calculada pela pontuação final × `fator_pausa`).

O limite é `LIMITE_CHUNK` em `src/piper_pipeline.rs`.

---

## Calibração por voz

Modelos Piper diferentes têm cadências diferentes. Para máxima naturalidade, recomendamos calibrar as pausas **uma vez por voz** e salvar num `<voz>.config.json`.

### Como calibrar

O princípio: sintetizar uma frase base sem pontuação, medir a duração, depois sintetizar a mesma frase com cada pontuação e medir o delta.

```python
# calibrador.py
import io, wave, json, os
import numpy as np
from piper import PiperVoice, SynthesisConfig

BASE = "o rato roeu a roupa do rei de roma e a rainha de raiva roeu o resto"
MODELOS = {
    ",": "o rato roeu, a roupa do rei de roma e a rainha de raiva roeu o resto",
    ";": "o rato roeu; a roupa do rei de roma e a rainha de raiva roeu o resto",
    ":": "o rato roeu: a roupa do rei de roma e a rainha de raiva roeu o resto",
    ".": "o rato roeu. a roupa do rei de roma e a rainha de raiva roeu o resto",
    "!": "o rato roeu! a roupa do rei de roma e a rainha de raiva roeu o resto",
    "?": "o rato roeu? a roupa do rei de roma e a rainha de raiva roeu o resto",
    "…": "o rato roeu… a roupa do rei de roma e a rainha de raiva roeu o resto",
}

def dur(voice, texto, ls=1.0):
    buf = io.BytesIO()
    with wave.open(buf, "wb") as wf:
        voice.synthesize_wav(texto, wf, syn_config=SynthesisConfig(length_scale=ls))
    buf.seek(0)
    with wave.open(buf, "rb") as wf:
        sr = wf.getframerate()
        a = np.frombuffer(wf.readframes(wf.getnframes()), dtype=np.int16)
    return sr, len(a) / sr

def calibrar(onnx_path, config_path, cache_path):
    if os.path.exists(cache_path):
        return json.load(open(cache_path))

    voice = PiperVoice.load(onnx_path, config_path=config_path)
    sr, dur_base = dur(voice, BASE)
    taxa = dur_base / len(BASE)

    pausas = {}
    for p, frase in MODELOS.items():
        _, d = dur(voice, frase)
        pausas[p] = max(0, int((d - dur_base) * 1000))

    cfg = {
        "voz": os.path.basename(onnx_path).replace(".onnx", ""),
        "sr": sr,
        "taxa_s_char_base": round(taxa, 5),
        "pausas_ms": pausas,
        "emocoes": {
            "neutro":    {"length_scale": 1.00, "fator_pausa": 1.00},
            "ansioso":   {"length_scale": 0.86, "fator_pausa": 0.40},
            "cansado":   {"length_scale": 1.25, "fator_pausa": 1.80},
            "triste":    {"length_scale": 1.08, "fator_pausa": 1.55},
            "empolgado": {"length_scale": 0.92, "fator_pausa": 0.55},
            "calmo":     {"length_scale": 1.05, "fator_pausa": 1.20},
        },
    }
    json.dump(cfg, open(cache_path, "w"), indent=2, ensure_ascii=False)
    return cfg
```

### Observação sobre o modelo Faber Medium

O Faber Medium **não emite pausas diferenciadas por pontuação**. O delta medido entre frases com/sem vírgula é próximo de zero. Nesse caso, use as `PAUSAS_BASE` default do `piper_pipeline.rs` (que são valores empíricos para narração pt-BR) e apenas module com `fator_pausa` da emoção.

### Cache

O `<voz>.config.json` é carregado instantaneamente. Próximas execuções pulam a calibração. Para forçar recalibração, apague o arquivo.

---

## Cliente Python (referência)

O worker devolve JSON pronto. O cliente Python só precisa renderizar os chunks.

```python
import subprocess, json, io, wave, os
import numpy as np
from piper import PiperVoice, SynthesisConfig

class VozzPiper:
    def __init__(self, worker_bin, onnx_path, config_path, tagger_data):
        env = {**os.environ, "BCDE_TAGGER_DATA": str(tagger_data)}
        self.proc = subprocess.Popen(
            [str(worker_bin)],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, bufsize=1, env=env)
        self._req({"action": "version"})   # handshake

        self.voice = PiperVoice.load(str(onnx_path), config_path=str(config_path))
        self.sr = self.voice.config.sample_rate

    def _req(self, obj):
        self.proc.stdin.write(json.dumps(obj) + "\n")
        self.proc.stdin.flush()
        return json.loads(self.proc.stdout.readline())

    def _synth_frag(self, ipa, ls):
        buf = io.BytesIO()
        with wave.open(buf, "wb") as wf:
            self.voice.synthesize_wav(f"[[ {ipa} ]]", wf,
                                      syn_config=SynthesisConfig(length_scale=ls))
        buf.seek(0)
        with wave.open(buf, "rb") as wf:
            sr = wf.getframerate()
            a = np.frombuffer(wf.readframes(wf.getnframes()), dtype=np.int16)
        return sr, a

    def sintetizar(self, texto, emocao="neutro"):
        r = self._req({
            "action": "process_piper_chunks",
            "text":   texto,
            "emocao": emocao,
        })
        chunks = r.get("chunks", [])
        if not chunks:
            return self.sr, np.zeros(0, dtype=np.int16)

        blocos = []
        for ch in chunks:
            ls = ch["length_scale"]
            for fr in ch["fragments"]:
                sr, audio = self._synth_frag(fr["ipa"], ls)
                blocos.append(audio)
                if fr["pausa_ms"] > 0:
                    blocos.append(np.zeros(int(sr * fr["pausa_ms"] / 1000),
                                           dtype=np.int16))
            if ch["pausa_apos_ms"] > 0:
                blocos.append(np.zeros(int(self.sr * ch["pausa_apos_ms"] / 1000),
                                       dtype=np.int16))

        return self.sr, np.concatenate(blocos)

    def fechar(self):
        try:
            self.proc.stdin.close()
            self.proc.wait(timeout=5)
        except Exception:
            self.proc.kill()
```

Uso:

```python
v = VozzPiper(
    worker_bin  = "vozz-g2p-rs/target/release/phonemizer-worker",
    onnx_path   = "pt_BR-faber-medium.onnx",
    config_path = "pt_BR-faber-medium.onnx.json",
    tagger_data = "BCDE-tagger/data",
)

sr, audio = v.sintetizar(
    "Ele se foi. Sem se despedir, sem dizer nada. Fiquei aqui. Sozinho.",
    emocao="triste",
)
```

---

## Comparador Python

O script `compare_vozz.py` faz uma comparação **palavra a palavra** entre três fonetizadores:

1. **vozz-g2p-js** — o Vozz original (JS)
2. **vozz-g2p-rs** — este projeto (Rust)
3. **espeak-ng pt-br** — referência de fato para TTS em português

Para cada par, gera estatísticas de divergência, agrupa por tipo (1-char-diff, acento-secundário, etc.) e produz um relatório legível + um léxico pronto para uso.

### Pré-requisitos

- Rust e o binário `phonemizer-worker` compilado
- `espeak-ng` instalado (`apt install espeak-ng`)
- Node.js 18+ (opcional, só para rodar o Vozz JS)
- Python 3.10+

### Uso

```bash
python3 compare_vozz.py corpus.txt
python3 compare_vozz.py corpus.txt --skip-js
python3 compare_vozz.py corpus.txt --force-all
python3 compare_vozz.py corpus.txt --no-test
```

### Saídas

- `cache/relatorio.txt` — relatório legível com categorias e exemplos
- `cache/lexico_espeak.json` — léxico IPA usando o espeak como gabarito
- `cache/exceptions.json` — siglas sem vogal descartadas
- `cache/tempos.json` — estatísticas de tempo
- `cache/js.json`, `cache/espeak.json`, `cache/rs.json` — saídas brutas

---

## API pública

### `g2p::fonemizar`

```rust
pub fn fonemizar(texto: &str, opcoes: &OpcoesFonemizar) -> String
```

Fonetiza um texto completo. Aplica sândi entre palavras. Resolve cada palavra pela ordem:

1. Homógrafo anotado pelo BCDE-tagger (se `tagger` + `homografos` passados)
2. `buscar_clitico_contexto` (contexto)
3. `buscar_lexico_contexto` (contexto)
4. `lexico_contexto` externo (espeak em contexto)
5. `lexico` externo (espeak isolado)
6. `buscar_lexico` / `buscar_clitico` (curados)
7. `palavra_para_ipa` (regras)

**Opções:**

```rust
pub struct OpcoesFonemizar<'a> {
    pub normalizar: bool,
    pub lexico: Option<&'a HashMap<String, String>>,
    pub lexico_contexto: Option<&'a HashMap<String, String>>,
    pub homografos: Option<&'a LexiconHomografos>,
    pub tagger: Option<&'a Tagger>,
}
```

### `g2p::palavra_para_ipa`

```rust
pub fn palavra_para_ipa(palavra: &str, proxima_inicial: Option<char>) -> String
```

Fonetiza uma única palavra **sem** passar pelo léxico nem normalização. Aplica trema automaticamente.

### `g2p::silabificar`

```rust
pub fn silabificar(palavra: &str) -> Vec<Silaba>
```

Devolve a estrutura silábica.

### `g2p::acentuar`

```rust
pub fn acentuar(silabas: &mut [Silaba], palavra: &str) -> i32
```

Marca a sílaba tônica **in place** e devolve o índice.

### `tagger::carregar_tagger`

```rust
pub fn carregar_tagger(dir: &str) -> Result<Tagger, String>
```

Carrega o BCDE-tagger do diretório de dados.

### `lexicon_homografos::LexiconHomografos`

```rust
impl LexiconHomografos {
    pub fn from_path(path: &Path) -> Result<Self, String>;
    pub fn ipa_para(&self, palavra: &str, sentido: &str) -> Option<&str>;
    pub fn tem_palavra(&self, palavra: &str) -> bool;
    pub fn len(&self) -> usize;
}
```

### `trema::u_pronunciado`

```rust
pub fn u_pronunciado(palavra: &str) -> bool
```

Diz se o `u` em `qu`/`gu` antes de `e`/`i` é pronunciado.

### `piper::ipa_para_piper`

```rust
pub fn ipa_para_piper(ipa: &str) -> Vec<String>
pub fn ipa_para_piper_str(ipa: &str) -> String
```

Converte IPA cru para o alfabeto do Piper.

### `piper_pipeline::preparar_chunks`

```rust
pub fn preparar_chunks(ipa_piper: &str, emocao: &str) -> Vec<Chunk>

pub struct Chunk {
    pub fragments: Vec<Fragmento>,
    pub length_scale: f32,
    pub pausa_apos_ms: u32,
}

pub struct Fragmento {
    pub ipa: String,
    pub pausa_ms: u32,
}
```

### `normalize::normalizar`

```rust
pub fn normalizar(texto: &str, opcoes: OpcoesNormalizar) -> String
```

Expande números, datas, horas, moedas, percentuais, temperaturas, abreviações e símbolos em palavras.

### `normalize::precisa_normalizar`

```rust
pub fn precisa_normalizar(palavra: &str) -> bool
```

Otimização: palavras puramente alfabéticas que não são abreviações nem siglas sem vogal pulam o normalizador.

### `normalize::eh_sigla_sem_vogal`

```rust
pub fn eh_sigla_sem_vogal(palavra: &str) -> bool
```

Detecta siglas em minúsculas sem vogais (`ldl`, `ngf`, `cpk`) e interjeições sem vogal (`pst`, `shh`, `hm`).

### `splitter::dividir_em_sentencas`

```rust
pub fn dividir_em_sentencas(texto: &str) -> Vec<String>
```

Divide um texto em sentenças, respeitando abreviações, decimais, siglas de uma letra e quebras de parágrafo.

### `numbers`

Módulo com funções de extenso. Todas aceitam `OpcoesExtenso { feminino: bool }`.

```rust
use vozz_g2p_rs::numbers::{
    inteiro_por_extenso, decimal_por_extenso, ordinal_por_extenso,
    soletrar_digitos, ano_por_extenso, OpcoesExtenso,
};
```

### `lexicon_palavra` e `lexicon_contexto`

Léxicos curados de palavras isoladas e contextuais.

```rust
use vozz_g2p_rs::lexicon_palavra::{buscar_lexico, buscar_clitico};
use vozz_g2p_rs::lexicon_contexto::{buscar_clitico_contexto, buscar_lexico_contexto};
```

---

## Regras fonológicas implementadas

### Nasalização

| Vogal | Antes de coda `m`/`n` | Antes de nasal no onset |
|-------|------------------------|--------------------------|
| `a`/`â` | `ɐ̃` | `ɐ̃` (tônico, ou pré-tônico antes de `nh`) / `æ` (pré-tônico antes de `m`/`n`) |
| `e` | `eɪ` | `e` puro |
| `i` | `i` puro | `i` puro |
| `o` | `o` puro | `o` puro |
| `u` | `ũ` | `ũ` (tônico, ou antes de `nh`) |

### Ditongos

**Decrescentes** (forte + fraca):

| Ditongo | IPA |
|---------|-----|
| `ai` | `aɪ` |
| `ei` | `eɪ` |
| `oi` | `oɪ` |
| `au` | `aʊ` |
| `eu` | `eʊ` |
| `éu` | `ɛʊ` |
| `iu` | `iw` |
| `ou` | `ow` |

**Crescentes** (fraca + forte): formam glide `jV`/`wV` (`história` → `ɾjæ`) **exceto** quando:

- A sílaba é tônica (`resumia` → `mˈiæ`).
- A segunda vogal tem acento gráfico.
- A consoante anterior é palatalizável (`cianeto`, `girasol`, `diádico`).

**Nasais gráficos**: `-ão`, `-ãe`, `-õe` formam um único núcleo.

### Trema (Acordo Ortográfico de 1990)

O trema foi abolido na grafia, mas a pronúncia do `u` em `qu`/`gu` antes de `e`/`i` continua. O módulo `trema.rs` reintroduz o trema hipotético e consulta:

1. Lista curada (`data/regras_trema.json` → `palavras_com_trema`).
2. Fallback por radical (`data/regras_trema.json` → `radicais`), que captura neologismos produtivos.

| Palavra | Antes | Agora |
|---------|-------|-------|
| `cinquenta` | `sĩˈkẽtɐ` ❌ | `sĩˈkwẽtɐ` ✅ |
| `tranquilo` | `tɾɐ̃ˈkilʊ` ❌ | `tɾɐ̃ˈkwilʊ` ✅ |
| `linguiça`  | `lĩˈɡisɐ` ❌ | `lĩˈɡwisɐ` ✅ |
| `quente`    | `ˈkẽtʃi` ✅ | `ˈkẽtʃi` ✅ |
| `guerra`    | `ˈɡɛxɐ` ✅ | `ˈɡɛxɐ` ✅ |

### Homógrafos

A desambiguação é feita pelo BCDE-tagger, que devolve `sense` por token. O IPA é buscado em `lexicon_homografos.json` pelo par `(palavra, sense)`.

Casos cobertos:

- `sede` como `seat` (local) vs `thirst` (vontade) — mesmo POS, sentidos distintos.
- `colher` como `spoon` (substantivo) vs `harvest` (verbo).
- `molho` como `sauce` vs `bundle`.
- `porto`, `sopro`, `rego`, `acerto`, `torno`, `engodo` — verbos 1sg vs substantivos.
- Expressões fixas como `pelo visto`, `pelo menos`, `pela manhã`.

### Outras

- **`x` intervocálico**: `/ʃ/` por padrão; `/ks/` em radicais eruditos (`taxi-`, `fix-`, `hidrox-`, `oxid-`, `sex-`, `nex-`).
- **Prefixo `ex-`**: `/z/` antes de vogal (`exame`, `exemplo`, `exato`).
- **`l` em coda**: `w` (final/átona); `ʊ` (tônica: `alto`, `palma`).
- **`r` em onset**: `x` no início absoluto e após coda nasal; `ɾ` intervocálico; `r` em ataque ramificado.
- **`c`/`g` brandos**: `s`/`ʒ` antes de `i`/`e`.
- **`d`/`t` africados**: `dʒ`/`tʃ` antes de `i`.
- **`s` intervocálico**: `z`.
- **`s`/`z` em coda**: sonorizam antes de consoante sonora.

### Sândi entre palavras

- `s`/`z` final sonorizam antes de vogal ou consoante sonora da próxima palavra.
- `r` final vira `ɾ` antes de vogal.
- `ʊ`/`y` final vira `w`/`j` antes de vogal (apenas quando o valor não veio de léxico contextual curado).

---

## Testes

```bash
cargo test
```

Cobre **~180 testes unitários** distribuídos em:

- `numbers`: extenso, decimais, ordinais, feminino, negativos.
- `lexicon_palavra`: lookup, case-insensitivity, chaves.
- `lexicon_contexto`: clíticos contextuais.
- `lexicon_homografos`: parsing do JSON, lookup.
- `normalize`: aspas, símbolos, números, datas, horas, moedas, siglas, abreviações.
- `splitter`: sentenças simples, abreviações, decimais, parágrafos.
- `trema`: hipótese, radicais, casos positivos/negativos.
- `piper`: conversão de chars, mapeamento `c→k`, ditongos.
- `piper_pipeline`: chunking curto/longo, segmentação por pausa, emoções.
- `g2p`: silabificação, acentuação, nasalização, ditongos, `x`, `ex-`, sândi.

---

## FAQ

**Por que Rust e não manter em JavaScript?**

Performance. Em palavras isoladas, o Rust é ~2,5x mais rápido que o JS (~16 µs vs ~40 µs por palavra). Além disso, um binário nativo integra melhor em pipelines Python, C++, Go, e em servidores de TTS.

**Por que o `espeak-ng` é o gabarito?**

Porque modelos neurais de TTS para pt-BR (Piper, Coqui, VITS) usam a convenção IPA do espeak-ng para treinar. Ser fiel ao espeak-ng significa ser fiel à convenção com que os modelos são treinados.

**Por que trocar o NB pelo BCDE-tagger?**

O BCDE-tagger resolve casos que o NB não pegava: homógrafos com o mesmo POS e sentidos distintos (ex.: `sede` = `seat`/`thirst`). Além disso, o tagger cobre todo o POS tagging, não só a desambiguação — o que ajuda na geração de features.

**Como a emoção afeta a síntese?**

Cada emoção define dois parâmetros: `length_scale` (velocidade da fala) e `fator_pausa` (multiplicador das pausas de pontuação). O `process_piper_chunks` aplica ambos e devolve os chunks já com as pausas calculadas. O cliente só renderiza.

**Preciso calibrar a voz?**

Não é obrigatório — os valores default em `piper_pipeline.rs` funcionam bem para narração pt-BR. Mas se você usar um modelo diferente do Faber Medium, vale rodar o calibrador e salvar `<voz>.config.json`.

**Como sei que o modelo de homógrafos não está overfitado?**

O BCDE-tagger foi validado em três corpora independentes com ~98% de acurácia POS. Veja a documentação do projeto para os números completos.

**Onde encontro o relatório completo de desenvolvimento?**

Em [`desenvolvimento/Relatorio.md`](https://github.com/bcdeosce/vozz-g2p-rs/blob/main/desenvolvimento/Relatorio.md).

---

## Licença

Este projeto é distribuído sob a **Apache License, Version 2.0**. Veja o arquivo [LICENSE](LICENSE) para o texto completo.

A escolha da Apache-2.0 mantém compatibilidade com o [Vozz original](https://github.com/Pedro21062014/vozz), que usa a mesma licença.

---

## Créditos

- **[Vozz](https://github.com/Pedro21062014/vozz)** — biblioteca original em JS, da qual este projeto é um fork. As regras fonológicas, o léxico base e o protocolo do worker vêm de lá.
- **[BCDE-tagger](https://github.com/bcdeosce/BCDE-tagger)** — POS tagger e desambiguador de homógrafos para pt-BR. Substitui o classificador NB anterior, elevando a acurácia em homógrafos com mesmo POS.
- **[espeak-ng](https://github.com/espeak-ng/espeak-ng)** — a referência de convenção IPA e de comportamento fonológico para pt-BR.
- **[Piper](https://github.com/rhasspy/piper)** — modelo neural de TTS que inspirou a busca pela máxima fidelidade ao `espeak-ng pt-br` e o pipeline de chunks + emoções.
- **[IME-USP](https://www.ime.usp.br/~pf/dicios/)** — a lista de palavras com trema (`br-com-trema-latin1.txt`), mantida pelo Prof. Paulo Feofiloff, serviu de base para o `data/regras_trema.json`. O arquivo original está em [https://www.ime.usp.br/~pf/dicios/br-com-trema-latin1.txt](https://www.ime.usp.br/~pf/dicios/br-com-trema-latin1.txt).
- **[Bifonia](https://github.com/TigreGotico/bifonia)** — referência arquitetural para o classificador NB da versão anterior (já removido).

---

## Contribuições

Pull requests são bem-vindos. Para mudanças de regra fonológica:

1. Rode `cargo test` para garantir que os testes existentes passam.
2. Rode `python3 compare_vozz.py corpus.txt --skip-install` para medir o impacto contra o espeak.
3. Adicione testes unitários cobrindo os novos casos.
4. Descreva no PR quais padrões foram corrigidos e qual a variação na taxa de acerto.

Para bug reports, inclua:

- Palavra(s) afetada(s)
- IPA produzido
- IPA esperado (do `espeak-ng -v pt-br --ipa=3 -q "palavra"`)
- Contexto (se for influenciado por palavras vizinhas)

---

## Notificação aos autores originais

Este projeto é um fork do [Vozz](https://github.com/Pedro21062014/vozz) e uma **adaptação arquitetural** do [Bifonia](https://github.com/TigreGotico/bifonia) (versão anterior).

Ambos os projetos originais são licenciados sob Apache 2.0, o que permite fork, modificação e redistribuição. Os autores foram notificados por cortesia profissional:

- **Vozz** (Pedro Berbis Freire): notificado via Issue no GitHub
- **Bifonia** (TigreGotico): notificado via `contact@tigregotico.pt`

Nenhum código do Bifonia foi reutilizado. A adaptação para Rust foi feita do zero, mas a arquitetura conceitual do classificador NB (posteriormente substituído pelo BCDE-tagger) vem do Bifonia.

O Vozz é o fork original do projeto. As regras fonológicas, o léxico base e o protocolo do worker vêm de lá. As correções feitas neste port estão documentadas na seção [Diferenças em relação ao Vozz original](#diferenças-em-relação-ao-vozz-original).

---

                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   [texto completo da licença Apache 2.0 — manter inalterado]

   Copyright 2024-presente, contribuidores do vozz-g2p-rs

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
```

