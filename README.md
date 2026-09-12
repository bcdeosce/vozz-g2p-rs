# vozz-g2p-rs

Conversor **grafema→fonema (G2P) para português do Brasil**, escrito em Rust puro, com foco em fidelidade à convenção IPA do `espeak-ng pt-br` — a mesma convenção com que modelos neurais de TTS (Piper, Coqui, VITS) são treinados.

Este projeto é um **fork/porte do [Vozz](https://github.com/Pedro21062014/vozz)**, uma biblioteca JavaScript de G2P para pt-BR. O objetivo é ter a mesma qualidade de fonetização em um binário nativo, sem dependência de runtime JS e com performance muito superior.

---

## Índice

- [Motivação](#motivação)
- [Diferenças em relação ao Vozz original](#diferenças-em-relação-ao-vozz-original)
- [Instalação](#instalação)
- [Uso como biblioteca Rust](#uso-como-biblioteca-rust)
- [Uso como worker (CLI)](#uso-como-worker-cli)
- [Comparador Python](#comparador-python)
- [API pública](#api-pública)
  - [g2p::fonemizar](#g2pfonemizar)
  - [g2p::palavra_para_ipa](#g2ppalavra_para_ipa)
  - [g2p::silabificar](#g2psilabificar)
  - [g2p::acentuar](#g2pacentuar)
  - [normalize::normalizar](#normalizenormalizar)
  - [normalize::precisa_normalizar](#normalizeprecisa_normalizar)
  - [normalize::eh_sigla_sem_vogal](#normalizeeh_sigla_sem_vogal)
  - [splitter::dividir_em_sentencas](#splitterdividir_em_sentencas)
  - [numbers](#numbers)
  - [lexicon](#lexicon)
- [Regras fonológicas implementadas](#regras-fonológicas-implementadas)
- [Testes](#testes)
- [Licença](#licença)
- [Créditos](#créditos)

---

## Motivação

O [Vozz](https://github.com/Pedro21062014/vozz) é uma biblioteca JS excelente para fonetização pt-BR, mas tem algumas limitações que motivaram este porte:

- **Performance**: o G2P em JS roda a ~21 µs/palavra. Em Rust, o mesmo pipeline roda a ~1,5 µs/palavra (14x mais rápido).
- **Integração**: um binário nativo é mais fácil de integrar em pipelines Python, C++, Go, ou em servidores de TTS de alta performance.
- **Precisão**: durante o porte, várias regras foram revisitadas com base em comparação direta contra o `espeak-ng pt-br`, corrigindo bugs do original.
- **Auditoria**: as regras estão em Rust fortemente tipado, com testes unitários cobrindo cada decisão fonológica.

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

O `vozz-g2p-rs` atinge **~90% de fidelidade** contra o `espeak-ng pt-br` no corpus de 336k palavras, contra ~50% do Vozz original.

---

## Instalação

### Requisitos

- Rust 1.70+ (`rustup` recomendado)
- (Opcional) `espeak-ng` — só para rodar o comparador Python
- (Opcional) Node.js 18+ — só para rodar o comparador Python contra o Vozz JS

### Build

```bash
git clone https://github.com/bcdeosce/vozz-g2p-rs.git
cd vozz-g2p-rs
cargo build --release
```

O binário fica em `./target/release/phonemizer-worker`.

### Como dependência (Cargo.toml)

```toml
[dependencies]
vozz-g2p-rs = { git = "https://github.com/bcdeosce/vozz-g2p-rs" }
```

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
};

let ipa = fonemizar("Rodamos em Kubernetes e Zendesk.", &opcoes);
println!("{}", ipa);
// xodˈɐ̃mʊs eɪŋ kubeɾnˈetʃis i zẽdˈɛski.
```

### Apenas uma palavra

```rust
use vozz_g2p_rs::palavra_para_ipa;

let ipa = palavra_para_ipa("exemplo", None);
println!("{}", ipa);
// ˌezˈeɪmplʊ
```

### Silabificação

```rust
use vozz_g2p_rs::silabificar;

for silaba in silabificar("banana") {
    println!("onset={:?} nucleo={:?} coda={:?} tonica={}",
        silaba.onset, silaba.nucleo, silaba.coda, silaba.tonica);
}
// onset=[] nucleo=["a"] coda=[] tonica=true   (tônica na 2ª sílaba)
// onset=["n"] nucleo=["a"] coda=[] tonica=false
// ...
```

### Normalização

```rust
use vozz_g2p_rs::normalizar;
use vozz_g2p_rs::normalize::OpcoesNormalizar;

let texto = normalizar("R$ 1.234,56 em 07/09/2025.", OpcoesNormalizar::default());
println!("{}", texto);
// mil duzentos e trinta e quatro reais e cinquenta e seis centavos em
// sete de setembro de dois mil e vinte e cinco.
```

### Divisão em sentenças

```rust
use vozz_g2p_rs::dividir_em_sentencas;

let sentencas = dividir_em_sentencas("Olá! Como vai? Bem, obrigado.");
for s in sentencas {
    println!("- {}", s);
}
// - Olá!
// - Como vai?
// - Bem, obrigado.
```

### Fonetização por sentença (pipeline completo)

```rust
use vozz_g2p_rs::{dividir_em_sentencas, fonemizar, normalizar};
use vozz_g2p_rs::normalize::OpcoesNormalizar;
use vozz_g2p_rs::g2p::OpcoesFonemizar;

let texto_bruto = "Em 2025, gastamos R$ 500,00. Foi muito!";
let normalizado = normalizar(texto_bruto, OpcoesNormalizar::default());
let sentencas = dividir_em_sentencas(&normalizado);

for s in &sentencas {
    let ipa = fonemizar(s, &OpcoesFonemizar::default());
    println!("{}", ipa);
}
```

---

## Uso como worker (CLI)

O `phonemizer-worker` é um processo persistente que lê JSON do `stdin` e escreve JSON no `stdout`, uma linha por requisição. O protocolo é compatível com o worker original do Vozz.

### Iniciar

```bash
./target/release/phonemizer-worker
```

### Requisições suportadas

**1. `version` — consulta versão e features**

```json
{"action": "version"}
```

Resposta:

```json
{"build": "2024-11-batch-v2", "features": ["set_lexicon", "process", "process_batch"]}
```

**2. `process` — fonetiza texto (com contexto de sentença)**

```json
{"action": "process", "text": "Olá, mundo! Como vai?", "voice": "", "overrides": {}}
```

Resposta:

```json
{
  "sentences": [
    {"text": "Olá, mundo!", "phonemes": "olˈa, mˈũŋdʊ!"},
    {"text": "Como vai?", "phonemes": "kˈomʊ vˈaɪ?"}
  ]
}
```

**3. `process_batch` — fonetiza uma lista de palavras isoladas**

Ideal para gerar léxicos em massa.

```json
{
  "action": "process_batch",
  "words": ["casa", "banana", "exemplo", "kubernetes"],
  "voice": "",
  "overrides": {}
}
```

Resposta:

```json
{
  "phonemes": {
    "casa": "kˈazæ",
    "banana": "bˌænˈɐ̃næ",
    "exemplo": "ˌezˈeɪmplʊ",
    "kubernetes": "kˌubeɾənˈetʃys"
  },
  "timing_ms": 0,
  "total": 4
}
```

**4. `set_lexicon` — carrega léxico externo em cache**

```json
{
  "action": "set_lexicon",
  "base_path": "/caminho/lexico_base.json",
  "global_path": "/caminho/overrides_globais.json",
  "voice_paths": {
    "idoso": "/caminho/lexico_idoso.json",
    "jovem": "/caminho/lexico_jovem.json"
  }
}
```

Cada arquivo JSON deve ser um objeto `{"palavra": "ipa", ...}`. O worker mantém 3 camadas de cache:
- `base` — léxico base
- `global` — overrides globais (têm precedência sobre base)
- `voice` — léxico específico por voz (tem precedência sobre global)

Ao processar, o worker escolhe o léxico específico da voz pedida; se não houver, usa o base.

### Uso com pipe

```bash
echo '{"action":"process","text":"Olá, mundo!","voice":"","overrides":{}}' \
  | ./target/release/phonemizer-worker
```

---

## Comparador Python

O script `compare_vozz.py` faz uma comparação **palavra a palavra** entre três fonetizadores:

1. **vozz-g2p-js** — o Vozz original (JS)
2. **vozz-g2p-rs** — este projeto (Rust)
3. **espeak-ng pt-br** — referência de fato para TTS em português

Para cada par, gera estatísticas de divergência, agrupa por tipo (1-char-diff, acento-secundario, etc.) e produz um relatório legível + um léxico pronto para uso.

### Pré-requisitos

- Rust e o binário `phonemizer-worker` compilado
- `espeak-ng` instalado (`apt install espeak-ng`)
- Node.js 18+ (opcional, só para rodar o Vozz JS)
- Python 3.10+

### Uso

```bash
# Rodada completa (js + rs + espeak)
python3 compare_vozz.py corpus.txt

# Iterar rápido: só compara vozz-rs contra espeak
python3 compare_vozz.py corpus.txt --skip-js

# Forçar regeração de tudo
python3 compare_vozz.py corpus.txt --force-all

# Pular cargo test (mais rápido)
python3 compare_vozz.py corpus.txt --no-test
```

### Fluxo

1. Roda `cargo test` (a menos que `--no-test`).
2. Verifica que o worker existe; compila se necessário.
3. Lê o corpus e extrai palavras únicas.
4. Filtra palavras que **não** precisam de normalização (alfabéticas, não abreviações, não siglas sem vogal).
5. Salva siglas sem vogal em `cache/exceptions.json` para revisão.
6. Roda Vozz JS (se necessário e se cache expirou).
7. Roda espeak-ng em 32 threads paralelas (se necessário).
8. Roda vozz-g2p-rs em batch (sempre).
9. Compara pares e gera:
   - `cache/relatorio.txt` — relatório legível com categorias e exemplos
   - `cache/lexico_espeak.json` — léxico IPA usando o espeak como gabarito
   - `cache/exceptions.json` — siglas sem vogal descartadas
   - `cache/tempos.json` — estatísticas de tempo
   - `cache/js.json`, `cache/espeak.json`, `cache/rs.json` — saídas brutas

### Cache

O script mantém cache dos resultados do Vozz JS e do espeak-ng entre execuções, com chave pelo hash do corpus. Recompilar o Rust não invalida o cache do JS/espeak, só o `rs.json` é regerado sempre.

---

## API pública

### `g2p::fonemizar`

```rust
pub fn fonemizar(texto: &str, opcoes: &OpcoesFonemizar) -> String
```

Fonetiza um texto completo. Tokeniza palavras, pontuação e espaços; aplica sândi (por exemplo, sonorização de `s` final antes de vogal). Resolve cada palavra pela ordem:

1. Léxico do usuário (overrides)
2. Léxico interno (`lexicon::LEXICO`)
3. Clíticos (`lexicon::CLITICOS`)
4. Regras do G2P (`palavra_para_ipa`)

**Parâmetros:**

- `texto`: entrada em pt-BR (não precisa estar normalizada; a função normaliza se `opcoes.normalizar` for `true`).
- `opcoes.normalizar` (default `true`): se `true`, expande números, datas, abreviações e símbolos antes de fonetizar.
- `opcoes.lexico`: mapa `palavra → IPA` com prioridade máxima. Útil para nomes próprios e termos técnicos.

**Retorno:** IPA em NFD com marcação de acento primário `ˈ` e secundário `ˌ`.

---

### `g2p::palavra_para_ipa`

```rust
pub fn palavra_para_ipa(palavra: &str, proxima_inicial: Option<char>) -> String
```

Fonetiza uma única palavra **sem** passar pelo léxico nem normalização. Aplica só o pipeline fonológico (segmentação → silabificação → acentuação → mapeamento). Use para inspecionar a decisão das regras.

**Parâmetros:**

- `palavra`: forma isolada em minúsculas.
- `proxima_inicial`: primeira letra da palavra seguinte. Se `Some` e for vogal ou consoante sonora, aplica sândi no `s` final.

---

### `g2p::silabificar`

```rust
pub fn silabificar(palavra: &str) -> Vec<Silaba>
```

Devolve a estrutura silábica da palavra como uma lista de `Silaba`, com `onset`, `nucleo`, `coda` separados. Não marca tonicidade — use `acentuar` para isso.

**Exemplo:**

```rust
let s = silabificar("prato");
assert_eq!(s[0].onset, vec!["p".to_string(), "r".to_string()]);
```

---

### `g2p::acentuar`

```rust
pub fn acentuar(silabas: &mut [Silaba], palavra: &str) -> i32
```

Marca a sílaba tônica **in place** e devolve o índice. Aplica, em ordem:

1. Acento gráfico (`á`, `é`, `í`, `ó`, `ú`, `â`, `ê`, `ô`) — manda absoluto.
2. Til na última sílaba (`ã`, `õ`) — tônico.
3. Sufixo `-is`/`-us` final sem acento gráfico → oxítona (verbos).
4. Terminações consonantais (`r`, `l`, `z`, `x`, `n`) → oxítona.
5. Ditongos decrescentes finais (`ai`, `ei`, `oi`, `au`, `eu`, `iu`, `ou`) → oxítona.
6. Caso geral: paroxítona (penúltima sílaba).

---

### `normalize::normalizar`

```rust
pub fn normalizar(texto: &str, opcoes: OpcoesNormalizar) -> String
```

Expande números, datas, horas, moedas, percentuais, temperaturas, abreviações e símbolos em palavras antes da fonetização.

**O que é expandido:**

| Entrada | Saída |
|---------|-------|
| `42` | `quarenta e dois` |
| `3,14` | `três vírgula quatorze` |
| `1.234` | `mil duzentos e trinta e quatro` |
| `07/09/2025` | `sete de setembro de dois mil e vinte e cinco` |
| `14:30` | `quatorze horas e trinta minutos` |
| `R$ 10` | `dez reais` |
| `50%` | `cinquenta por cento` |
| `30°C` | `trinta graus celsius` |
| `1º` | `primeiro` |
| `1ª` | `primeira` |
| `Sr.` | `senhor` |
| `Dra.` | `doutora` |
| `kg` | `quilogramas` |
| `%` | ` por cento ` |
| `&` | ` e ` |

**Opções:**

- `expandir_numeros` (default `true`): expande números e decimais.
- `expandir_siglas` (default `true`): soletra siglas (`XYZ` → `xis ípsilon zê`).

---

### `normalize::precisa_normalizar`

```rust
pub fn precisa_normalizar(palavra: &str) -> bool
```

Diz se uma palavra precisa passar pelo normalizador. Retorna `false` para palavras puramente alfabéticas que não são abreviações nem siglas sem vogal — nesse caso, o normalizador não faria nenhuma substituição e pode ser pulado.

Usada pelo worker como otimização: palavras como `casa`, `banana`, `exemplo` pulam o normalizador e vão direto para o G2P.

**Exemplos:**

```rust
assert!(!precisa_normalizar("casa"));
assert!(!precisa_normalizar("banana"));
assert!(precisa_normalizar("dr"));
assert!(precisa_normalizar("100"));
assert!(precisa_normalizar("50%"));
assert!(precisa_normalizar("ldl"));  // sigla sem vogal
```

---

### `normalize::eh_sigla_sem_vogal`

```rust
pub fn eh_sigla_sem_vogal(palavra: &str) -> bool
```

Detecta siglas em minúsculas que chegaram ao corpus sem vogais, como `ldl`, `ngf`, `cpk`, `vldl`. Também cobre interjeições sem vogal (`pst`, `shh`, `hm`). Essas palavras o G2P não consegue fonemizar corretamente (o tokenizador as trata como sequências sem núcleo).

**Regras:**

- 2 a 6 caracteres
- Todos alfabéticos (sem dígitos, símbolos ou hífen)
- Nenhum caractere é vogal do português (incluindo `á`, `â`, `ã`, `é`, `ê`, `í`, `ó`, `ô`, `õ`, `ú`, `ü`)

O worker as descarta da fonetização e o `compare_vozz.py` as salva em `exceptions.json` para revisão manual.

---

### `splitter::dividir_em_sentencas`

```rust
pub fn dividir_em_sentencas(texto: &str) -> Vec<String>
```

Divide um texto completo em sentenças, respeitando:

- **Abreviações**: `Sr.`, `Dra.`, `etc.` não encerram sentença.
- **Decimais**: `3.14` não encerra.
- **Siglas de uma letra**: `A. Silva` não encerra.
- **Continuação em minúscula**: `Ele disse: Oi! e saiu.` é uma sentença só.
- **Quebra de parágrafo**: `\n\n` sempre encerra.

**Exemplo:**

```rust
let s = dividir_em_sentencas("Olá! Como vai? Bem, obrigado.");
assert_eq!(s, vec!["Olá!", "Como vai?", "Bem, obrigado."]);
```

---

### `numbers`

Módulo com funções de extenso. Todas aceitam `OpcoesExtenso { feminino: bool }`.

```rust
use vozz_g2p_rs::numbers::{
    inteiro_por_extenso, decimal_por_extenso, ordinal_por_extenso,
    soletrar_digitos, ano_por_extenso, OpcoesExtenso,
};

assert_eq!(inteiro_por_extenso(1998, OpcoesExtenso::default()),
           "mil novecentos e noventa e oito");

assert_eq!(decimal_por_extenso(3, "14", OpcoesExtenso::default()),
           "três vírgula quatorze");

assert_eq!(ordinal_por_extenso(42, OpcoesExtenso::default()),
           "quadragésimo segundo");

assert_eq!(soletrar_digitos("123"), "um dois três");
```

---

### `lexicon`

Léxico interno com ~180 entradas curadas: palavras funcionais de alta frequência, vogais tônicas abertas/fechadas irregulares, estrangeirismos comuns e siglas.

```rust
use vozz_g2p_rs::lexicon::{buscar_lexico, buscar_clitico};

assert_eq!(buscar_lexico("não"), Some("nˈɐ̃ʊ̃"));
assert_eq!(buscar_clitico("de"), Some("dʒy"));
```

Os mapas `LEXICO` e `CLITICOS` são expostos como `Lazy<HashMap>` e podem ser inspecionados diretamente.

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
- A segunda vogal tem acento gráfico (`confiávamos` → `fiˈa...`).
- A consoante anterior é palatalizável (`cianeto`, `girasol`, `diádico`).

**Nasais gráficos**: `-ão`, `-ãe`, `-õe` formam um único núcleo.

### Outras

- **`x` intervocálico**: `/ʃ/` por padrão; `/ks/` em radicais eruditos.
- **Prefixo `ex-`**: `/z/` antes de vogal.
- **`l` em coda**: `w` (final/átona); `ʊ` (tônica).
- **`r` em onset**: `x` no início absoluto e após coda nasal; `ɾ` intervocálico; `r` em ataque ramificado.
- **`c`/`g` brandos**: `s`/`ʒ` antes de `i`/`e`.
- **`d`/`t` africados**: `dʒ`/`tʃ` antes de `i` (final de sílaba).
- **`s` intervocálico**: `z`.
- **`s`/`z` em coda**: sonorizam antes de consoante sonora.

### Sândi entre palavras

- `s`/`z` final sonorizam antes de vogal ou consoante sonora da próxima palavra.

---

## Testes

```bash
cargo test
```

Cobre **~150 testes unitários** distribuídos em:

- `numbers`: extenso (unidades, dezenas, centenas, milhares, negativos, feminino, decimais, ordinais, dígitos, anos).
- `lexicon`: lookup, case-insensitivity, NFD, sem `g` ASCII, chaves sem underscore.
- `normalize`: aspas, símbolos, números, datas, horas, moedas, temperatura, siglas, abreviações, `precisa_normalizar`, `eh_sigla_sem_vogal`.
- `splitter`: sentenças simples, abreviações, decimais, parágrafos, reticências, aspas.
- `g2p`: silabificação, acentuação, mapeamento de onset/núcleo/coda, ditongos, nasalização, regra do `x`, prefixo `ex-`, sândi, etc.

---

## Licença

Este projeto é distribuído sob a **Apache License, Version 2.0**. Veja o arquivo [LICENSE](LICENSE) para o texto completo.

A escolha da Apache-2.0 mantém compatibilidade com o [Vozz original](https://github.com/Pedro21062014/vozz), que usa a mesma licença.

---

## Créditos

- **[Vozz](https://github.com/Pedro21062014/vozz)** — biblioteca original em JS, da qual este projeto é um fork. As regras fonológicas, o léxico base e o protocolo do worker vêm de lá.
- **[espeak-ng](https://github.com/espeak-ng/espeak-ng)** — a referência de convenção IPA e de comportamento fonológico para pt-BR. Sem ele, este projeto não existiria.
- **[Piper](https://github.com/rhasspy/piper)** — modelo neural de TTS que inspirou a busca pela máxima fidelidade ao `espeak-ng pt-br`.

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

                                 Apache License
                           Version 2.0, January 2004
                        http://www.apache.org/licenses/

   TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

   1. Definitions.

      "License" shall mean the terms and conditions for use, reproduction,
      and distribution as defined by Sections 1 through 9 of this document.

      "Licensor" shall mean the copyright owner or entity authorized by
      the copyright owner that is granting the License.

      "Legal Entity" shall mean the union of the acting entity and all
      other entities that control, are controlled by, or are under common
      control with that entity. For the purposes of this definition,
      "control" means (i) the power, direct or indirect, to cause the
      direction or management of such entity, whether by contract or
      otherwise, or (ii) ownership of fifty percent (50%) or more of the
      outstanding shares, or (iii) beneficial ownership of such entity.

      "You" (or "Your") shall mean an individual or Legal Entity
      exercising permissions granted by this License.

      "Source" form shall mean the preferred form for making modifications,
      including but not limited to software source code, documentation
      source, and configuration files.

      "Object" form shall mean any form resulting from mechanical
      transformation or translation of a Source form, including but
      not limited to compiled object code, generated documentation,
      and conversions to other media types.

      "Work" shall mean the work of authorship, whether in Source or
      Object form, made available under the License, as indicated by a
      copyright notice that is included in or attached to the work
      (an example is provided in the Appendix below).

      "Derivative Works" shall mean any work, whether in Source or Object
      form, that is based on (or derived from) the Work and for which the
      editorial revisions, annotations, elaborations, or other modifications
      represent, as a whole, an original work of authorship. For the purposes
      of this License, Derivative Works shall not include works that remain
      separable from, or merely link (or bind by name) to the interfaces of,
      the Work and Derivative Works thereof.

      "Contribution" shall mean any work of authorship, including
      the original version of the Work and any modifications or additions
      to that Work or Derivative Works thereof, that is intentionally
      submitted to Licensor for inclusion in the Work by the copyright owner
      or by an individual or Legal Entity authorized to submit on behalf of
      the copyright owner. For the purposes of this definition, "submitted"
      means any form of electronic, verbal, or written communication sent
      to the Licensor or its representatives, including but not limited to
      communication on electronic mailing lists, source code control systems,
      and issue tracking systems that are managed by, or on behalf of, the
      Licensor for the purpose of discussing and improving the Work, but
      excluding communication that is conspicuously marked or otherwise
      designated in writing by the copyright owner as "Not a Contribution."

      "Contributor" shall mean Licensor and any individual or Legal Entity
      on behalf of whom a Contribution has been received by Licensor and
      subsequently incorporated within the Work.

   2. Grant of Copyright License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      copyright license to reproduce, prepare Derivative Works of,
      publicly display, publicly perform, sublicense, and distribute the
      Work and such Derivative Works in Source or Object form.

   3. Grant of Patent License. Subject to the terms and conditions of
      this License, each Contributor hereby grants to You a perpetual,
      worldwide, non-exclusive, no-charge, royalty-free, irrevocable
      (except as stated in this section) patent license to make, have made,
      use, offer to sell, sell, import, and otherwise transfer the Work,
      where such license applies only to those patent claims licensable
      by such Contributor that are necessarily infringed by their
      Contribution(s) alone or by combination of their Contribution(s)
      with the Work to which such Contribution(s) was submitted. If You
      institute patent litigation against any entity (including a
      cross-claim or counterclaim in a lawsuit) alleging that the Work
      or a Contribution incorporated within the Work constitutes direct
      or contributory patent infringement, then any patent licenses
      granted to You under this License for that Work shall terminate
      as of the date such litigation is filed.

   4. Redistribution. You may reproduce and distribute copies of the
      Work or Derivative Works thereof in any medium, with or without
      modifications, and in Source or Object form, provided that You
      meet the following conditions:

      (a) You must give any other recipients of the Work or
          Derivative Works a copy of this License; and

      (b) You must cause any modified files to carry prominent notices
          stating that You changed the files; and

      (c) You must retain, in the Source form of any Derivative Works
          that You distribute, all copyright, patent, trademark, and
          attribution notices from the Source form of the Work,
          excluding those notices that do not pertain to any part of
          the Derivative Works; and

      (d) If the Work includes a "NOTICE" text file as part of its
          distribution, then any Derivative Works that You distribute must
          include a readable copy of the attribution notices contained
          within such NOTICE file, excluding those notices that do not
          pertain to any part of the Derivative Works, in at least one
          of the following places: within a NOTICE text file distributed
          as part of the Derivative Works; within the Source form or
          documentation, if provided along with the Derivative Works; or,
          within a display generated by the Derivative Works, if and
          wherever such third-party notices normally appear. The contents
          of the NOTICE file are for informational purposes only and
          do not modify the License. You may add Your own attribution
          notices within Derivative Works that You distribute, alongside
          or as an addendum to the NOTICE text from the Work, provided
          that such additional attribution notices cannot be construed
          as modifying the License.

      You may add Your own copyright statement to Your modifications and
      may provide additional or different license terms and conditions
      for use, reproduction, or distribution of Your modifications, or
      for any such Derivative Works as a whole, provided Your use,
      reproduction, and distribution of the Work otherwise complies with
      the conditions stated in this License.

   5. Submission of Contributions. Unless You explicitly state otherwise,
      any Contribution intentionally submitted for inclusion in the Work
      by You to the Licensor shall be under the terms and conditions of
      this License, without any additional terms or conditions.
      Notwithstanding the above, nothing herein shall supersede or modify
      the terms of any separate license agreement you may have executed
      with Licensor regarding such Contributions.

   6. Trademarks. This License does not grant permission to use the trade
      names, trademarks, service marks, or product names of the Licensor,
      except as required for reasonable and customary use in describing the
      origin of the Work and reproducing the content of the NOTICE file.

   7. Disclaimer of Warranty. Unless required by applicable law or
      agreed to in writing, Licensor provides the Work (and each
      Contributor provides its Contributions) on an "AS IS" BASIS,
      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
      implied, including, without limitation, any warranties or conditions
      of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
      PARTICULAR PURPOSE. You are solely responsible for determining the
      appropriateness of using or redistributing the Work and assume any
      risks associated with Your exercise of permissions under this License.

   8. Limitation of Liability. In no event and under no legal theory,
      whether in tort (including negligence), contract, or otherwise,
      unless required by applicable law (such as deliberate and grossly
      negligent acts) or agreed to in writing, shall any Contributor be
      liable to You for damages, including any direct, indirect, special,
      incidental, or consequential damages of any character arising as a
      result of this License or out of the use or inability to use the
      Work (including but not limited to damages for loss of goodwill,
      work stoppage, computer failure or malfunction, or any and all
      other commercial damages or losses), even if such Contributor
      has been advised of the possibility of such damages.

   9. Accepting Warranty or Additional Liability. While redistributing
      the Work or Derivative Works thereof, You may choose to offer,
      and charge a fee for, acceptance of support, warranty, indemnity,
      or other liability obligations and/or rights consistent with this
      License. However, in accepting such obligations, You may act only
      on Your own behalf and on Your sole responsibility, not on behalf
      of any other Contributor, and only if You agree to indemnify,
      defend, and hold each Contributor harmless for any liability
      incurred by, or claims asserted against, such Contributor by reason
      of your accepting any such warranty or additional liability.

   END OF TERMS AND CONDITIONS

   APPENDIX: How to apply the Apache License to your work.

      To apply the Apache License to your work, attach the following
      boilerplate notice, with the fields enclosed by brackets "[]"
      replaced with your own identifying information. (Don't include
      the brackets!)  The text should be enclosed in the appropriate
      comment syntax for the file format. We also recommend that a
      file or class name and description of purpose be included on the
      same "printed page" as the copyright notice for easier
      identification within third-party archives.

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
