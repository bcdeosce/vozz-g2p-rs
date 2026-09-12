Aqui está o mesmo raciocínio aplicado a cada símbolo da tabela:

## Tabela completa

| Símbolo | Decomposição Unicode | Espeak pt-br | Exemplo |
|---|---|---|---|
| `ɪ` | `i` + U+031E (lowered) = `i̞` | `ɪ` (mantém) | `pai` → `pˈaɪ` |
| `ɦ` | `h` + U+032C (voiced) = `h̬` | não ocorre | — |
| `ɽ` | `ɾ` + U+0322 (retroflex) = `ɾ̢` | não ocorre | — |
| `ɫ` | `l` + U+02E0 (velarized) = `lˠ` | não ocorre | — |
| `ẽ` | `e` + U+0303 (nasal) = `ẽ` | `eɪ` em coda / `e` em onset | `tempo` → `tˈeɪmpʊ` |
| `ĩ` | `i` + U+0303 (nasal) = `ĩ` | `iŋ` em coda / `i` em onset | `sim` → `sˈiŋ` |
| `ə` | símbolo único (U+0259) | não ocorre | — |

## Explicação de cada um

### `ɪ`

**Fonologia:** vogal quase-fechada quase-anterior não-arredondada.

**Decomposição:** `i` + U+031E (lowered) = `i̞`. É um `i` "abaixado".

**Espeak pt-br:** mantém `ɪ` direto. Não tem transformação. Aparece como offglide em ditongos decrescentes.

```
pai    → pˈaɪ
sei    → sˈeɪ
foi    → fˈoɪ
```

### `ɦ`

**Fonologia:** fricativa glotal sonora.

**Decomposição:** `h` + U+032C (voiced) = `h̬`. É um `h` com voz.

**Espeak pt-br:** não ocorre. O pt-BR não tem `h` sonoro.

### `ɽ`

**Fonologia:** flap retroflexo.

**Decomposição:** `ɾ` + U+0322 (retroflex) = `ɾ̢`. É o `ɾ` com a língua virada pra trás.

**Espeak pt-br:** não ocorre. O pt-BR padrão usa `ɾ` alveolar. Alguns dialetos (interior de SP, sul) usam `ɽ`, mas o espeak pt-br não modela.

### `ɫ`

**Fonologia:** lateral velarizada (o "l" escuro do inglês em `milk`).

**Decomposição:** `l` + U+02E0 (velarized) = `lˠ`. É um `l` com articulação secundária velar.

**Espeak pt-br:** não ocorre. O pt-BR usa `l` claro em onset (`lua`) e `w` ou `ʊ` em coda (`sal`, `alto`).

### `ẽ`

**Fonologia:** vogal média-anterior não-arredondada nasalizada.

**Decomposição:** `e` + U+0303 (nasal) = `ẽ`. É um `e` com nasalidade.

**Espeak pt-br:** **não usa** `ẽ`. Quando o `e` está antes de nasal em coda, o espeak produz `eɪ` (ditongo oral-nasal). Quando está em contato com nasal no onset, produz `e` puro.

```
tempo  → tˈeɪmpʊ    (coda nasal → eɪ)
vem    → vˈeɪŋ      (coda nasal → eɪ)
tenho  → tˈeɲʊ      (onset nasal → e)
```

A equivalência que você quer é:

```
ẽ = eɪ     (em coda)
ẽ = e      (em onset)
```

### `ĩ`

**Fonologia:** vogal fechada-anterior não-arredondada nasalizada.

**Decomposição:** `i` + U+0303 (nasal) = `ĩ`. É um `i` com nasalidade.

**Espeak pt-br:** **não usa** `ĩ`. Quando o `i` está antes de nasal em coda, o espeak produz `i` + consoante de apoio (`ŋ` ou `m`). Quando está em contato com nasal no onset, produz `i` puro.

```
sim    → sˈiŋ      (coda nasal → iŋ)
vim    → vˈiŋ      (coda nasal → iŋ)
cinco  → sˈiŋkʊ    (coda nasal → iŋ)
vinte  → vˈiŋtʃi   (coda nasal → iŋ)
vinho  → vˈiɲʊ     (onset nasal → i, não nasaliza)
```

A equivalência:

```
ĩ = iŋ     (em coda, antes de não-labial)
ĩ = im     (em coda, antes de p/b/m)
ĩ = i      (em onset)
```

### `ə`

**Fonologia:** vogal média-central (schwa).

**Decomposição:** não tem. `ə` é um símbolo base por si só (U+0259). Não existe como `base + diacrítico`.

**Espeak pt-br:** não ocorre. O pt-BR reduz vogais átonas para `y` (`e` final), `ʊ` (`o` final), `æ` (`a` final), mas não para `ə`.

```
leite  → lˈeɪtʃy    (e final átono → y)
livro  → lˈivɾʊ     (o final átono → ʊ)
cama   → kˈɐ̃mæ     (a final átono → æ)
```

## Resumo

**Símbolos que o Vozz usa direto:** `ɪ`, `ʊ`, `ɐ̃`, `ũ`, `õ`, `ẽ` (não, esse não), `ĩ` (não, esse também não).

**Símbolos que o espeak nunca usa:** `ɦ`, `ɽ`, `ɫ`, `ə`.

**Símbolos que o espeak substitui por outra coisa:**

| Se você espera | Espeak pt-br produz |
|---|---|
| `ĩ` | `iŋ` (coda não-labial), `im` (coda labial), `i` (onset) |
| `ẽ` | `eɪ` (coda), `e` (onset) |
| `ɐ̃` | mantém `ɐ̃` |
| `ũ` | mantém `ũ` |

O padrão é o mesmo do `sĩ = sˈiŋ`: a vogal nasal vira **vogal oral + consoante de apoio** quando está em coda. Se estiver em onset, a vogal fica pura.
