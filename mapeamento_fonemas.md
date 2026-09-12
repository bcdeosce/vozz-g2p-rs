Segue o mapa completo, organizado por categoria. Cada linha tem: **entrada** (grafema) → **saída IPA** → **contexto/condição** → **onde está no código**.

---

## 1. Vogais — orais (núcleo simples)

| Grafema | IPA | Contexto | Onde |
|---|---|---|---|
| `a` | `a` | tônico ou pré-tônico não-final | `vogal_oral` |
| `a` | `æ` | final átono, pré-tônico antes de nasal, pós-tônico | `vogal_oral` |
| `á`, `à` | `a` | sempre | `vogal_oral` |
| `â` | `ɐ` | sempre | `vogal_oral` |
| `ã` | `ɐ̃` | sempre | `vogal_oral` |
| `e` | `e` | tônico ou pré-tônico | `vogal_oral` |
| `e` | `y` | final átono | `vogal_oral` |
| `é` | `ɛ` | sempre | `vogal_oral` |
| `ê` | `e` | sempre | `vogal_oral` |
| `i`, `í` | `i` | sempre | `vogal_oral` |
| `o` | `o` | tônico ou pré-tônico | `vogal_oral` |
| `o` | `ʊ` | final átono | `vogal_oral` |
| `ó` | `ɔ` | sempre | `vogal_oral` |
| `ô` | `o` | sempre | `vogal_oral` |
| `õ` | `õ` | sempre | `vogal_oral` |
| `u`, `ú`, `ü` | `u` | sempre | `vogal_oral` |

## 2. Vogais — nasais (núcleo com coda `m`/`n` ou contato nasal)

| Grafema | IPA | Condição | Onde |
|---|---|---|---|
| `a`, `á`, `à`, `â`, `ã` | `ɐ̃` | coda `m`/`n` ou contato nasal tônico | `nucleo_nasal` |
| `e`, `é`, `ê` | `eɪ` | coda `m`/`n` | `nucleo_nasal` |
| `e`, `é`, `ê` | `e` | contato nasal (não coda) | `nucleo_nasal` |
| `i`, `í` | `i` | sempre (nunca nasaliza) | `nucleo_nasal` |
| `o`, `ó`, `ô` | `o` | sempre (nunca nasaliza) | `nucleo_nasal` |
| `õ` | `õ` | sempre (til gráfico) | `nucleo_nasal` |
| `u`, `ú` | `ũ` | coda `m`/`n` ou antes de `nh` ou tônico antes de `m`/`n` | `nucleo_nasal` |

## 3. Ditongos decrescentes (forte + fraca)

| Entrada | IPA | Condição | Onde |
|---|---|---|---|
| `ai` | `aɪ` | oral | `offglide` |
| `ai` + coda nasal | `ɐ̃ɪ̃` | nasal | `offglide` |
| `ei` | `eɪ` | oral | `offglide` |
| `ei` + coda nasal | `eɪ̃` | nasal | `offglide` |
| `oi` | `oɪ` | oral | `offglide` |
| `ou` | `ow` | oral (após `o`) | `offglide` |
| `au` | `aʊ` | oral (após `a`) | `offglide` |
| `eu` | `eʊ` | oral (após `e`) | `offglide` |
| `éu` | `ɛʊ` | oral (após `é`) | `offglide` |
| `iu` | `iw` | oral (após `i`) | `offglide` |
| `ãe` | `ɐ̃ɪ̃` | nasal gráfico | `mapear_nucleo` |
| `ão` | `ɐ̃ʊ̃` | nasal gráfico | `mapear_nucleo` |
| `õe` | `õɪ̃` | nasal gráfico | `mapear_nucleo` |
| `-am` final | `ɐ̃ʊ̃` | verbo, override | `palavra_para_ipa` |
| `-em`, `-ens` | `eɪ`, `eɪs` | normal (paroxítona) | `acentuar` |

## 4. Ditongos crescentes (fraca + forte) e hiatos

| Entrada | IPA | Condição | Onde |
|---|---|---|---|
| `ia`, `io`, `ie`, `iu` | `jV` | átono, sem acento gráfico na 2ª vogal | `mapear_nucleo` |
| `ia`, `io` | `i.V` | tônico, sem acento na 2ª vogal | `mapear_nucleo` |
| `iá`, `ió`, `iá` | `i.ˈa` | 2ª vogal com acento gráfico | `mapear_nucleo` |
| `ciV`, `giV`, `diV`, `tiV` | `siV`, `ʒiV`, `dʒiV`, `tʃiV` | consoante palatalizável + `i` + vogal | `silabificar` |
| `bliV`, `pliV`, etc. | `bli.V` | 2ª vogal acentuada | `mapear_nucleo` |
| `iu` + consoante + vogal | `ju` | `iu` antes de consoante | (revertido) |

## 5. Onset — consoantes simples

| Grafema | IPA | Condição | Onde |
|---|---|---|---|
| `b` | `b` | sempre | `mapear_onset` |
| `c` | `k` | antes de `a`, `o`, `u`, consoante | `mapear_onset` |
| `c` | `s` | antes de `e`, `i` | `mapear_onset` |
| `ç` | `s` | sempre | `mapear_onset` |
| `d` | `d` | antes de `a`, `o`, `u`, consoante | `mapear_onset` |
| `d` | `dʒ` | antes de `i`, `e` (brando) | `mapear_onset` |
| `f` | `f` | sempre | `mapear_onset` |
| `g` | `ɡ` | antes de `a`, `o`, `u`, consoante | `mapear_onset` |
| `g` | `ʒ` | antes de `e`, `i` (brando) | `mapear_onset` |
| `h` | ∅ | sempre mudo | `mapear_onset` |
| `j` | `ʒ` | sempre | `mapear_onset` |
| `k` | `k` | sempre | `mapear_onset` |
| `l` | `l` | sempre em onset | `mapear_onset` |
| `m` | `m` | sempre em onset | `mapear_onset` |
| `n` | `n` | sempre em onset | `mapear_onset` |
| `p` | `p` | sempre | `mapear_onset` |
| `q` | `k` | sempre em onset | `mapear_onset` |
| `t` | `t` | antes de `a`, `o`, `u` | `mapear_onset` |
| `t` | `tʃ` | antes de `i`, `e` (brando) | `mapear_onset` |
| `v` | `v` | sempre | `mapear_onset` |
| `w` | `w` | sempre (estrangeirismo) | `mapear_onset` |
| `x` | `ʃ` | padrão | `mapear_onset` |
| `x` | `z` | prefixo `ex-` + vogal | `palavra_para_ipa` |
| `x` | `ks` | radicais eruditos | `tem_radical_ks` |
| `y` | `j` | sempre | `mapear_onset` |
| `z` | `z` | sempre | `mapear_onset` |

## 6. Onset — `r`

| Contexto | IPA | Onde |
|---|---|---|
| início absoluto de palavra (`rato`) | `x` | `mapear_onset` |
| após coda consonantal (`israel`) | `x` | `mapear_onset` |
| intervocálico (`caro`) | `ɾ` | `mapear_onset` |
| ataque ramificado (`prato`, `bravo`) | `r` | `mapear_onset` |

## 7. Onset — `s`

| Contexto | IPA | Onde |
|---|---|---|
| intervocálico (`casa`) | `z` | `mapear_onset` |
| início, após consoante | `s` | `mapear_onset` |

## 8. Onset — dígrafos

| Grafema | IPA | Condição | Onde |
|---|---|---|---|
| `ch` | `ʃ` | sempre | `mapear_onset` |
| `lh` | `lj` | sempre | `mapear_onset` |
| `nh` | `ɲ` | sempre | `mapear_onset` |
| `rr` | `x` | sempre | `mapear_onset` |
| `ss` | `s` | sempre | `mapear_onset` |
| `gu` + `e` | `ɡ` | digrafo | `segmentar` |
| `qu` + `e`/`i` (início) | `k` | digrafo | `segmentar` |
| `qu` + `e`/`i` (meio) | `k` | (revertido) | `segmentar` |
| `gu` + `a`/`o`/`i` | `ɡw` | glide | `segmentar` |
| `qu` + `a`/`o` | `kw` | glide | `segmentar` |

## 9. Coda — consoantes simples

| Grafema | IPA | Condição | Onde |
|---|---|---|---|
| `m`, `n` | `m` | antes de `p`, `b`, `m` | `consoante_nasal_coda` |
| `m`, `n` | `ŋ` | qualquer outra | `consoante_nasal_coda` |
| `r` | `r` | final de palavra | `mapear_coda` |
| `r` | `ɾə` | antes de consoante | `mapear_coda` |
| `l` | `l` | final após `o`/`ó`/`ô` | `mapear_coda` |
| `l` | `w` | final após outras vogais | `mapear_coda` |
| `l` | `ʊ` | antes de consoante após `a`/`e` | `mapear_coda` |
| `l` | `w` | antes de consoante após `i`/`o`/`u` | `mapear_coda` |
| `s`, `ss` | `z` | final sonoro | `mapear_coda` |
| `s`, `ss` | `s` | final surdo | `mapear_coda` |
| `s`, `ss` | `z` | antes de `b`,`d`,`g`,`j`,`l`,`m`,`n`,`r`,`v`,`z`,`ç` | `mapear_coda` |
| `s`, `ss` | `s` | demais | `mapear_coda` |
| `z` | `z` | interno | `mapear_coda` |
| `z` | `s` | final surdo | `mapear_coda` |
| `x` | `s` | sempre em coda | `mapear_coda` |
| `c` | `k` | sempre em coda | `mapear_coda` |
| `ç` | `s` | sempre em coda | `mapear_coda` |
| `b` | `b` | coda | `mapear_coda` |
| `d` | `d` | coda (não africado) | `mapear_coda` |
| `g` | `ɡ` | coda | `mapear_coda` |
| `p` | `p` | coda | `mapear_coda` |
| `t` | `tʃ` | coda | `mapear_coda` |
| `ch` | `ʃ` | coda | `mapear_coda` |

## 10. Coda — clusters ramificados

| Entrada | IPA | Onde |
|---|---|---|
| `pr`, `br`, `tr`, `dr`, `kr`, `gr`, `fr`, `vr` | onset ramificado válido | `silabificar` |
| `pl`, `bl`, `kl`, `gl`, `fl` | onset ramificado válido | `silabificar` |
| outras combinações | coda + onset separados | `silabificar` |

## 11. Acentuação (posição da tônica)

| Terminação | Tipo | Onde |
|---|---|---|
| acento gráfico (`á`,`é`,`í`,`ó`,`ú`,`â`,`ê`,`ô`) | tônica na sílaba marcada | `acentuar` |
| `ã`, `õ` na última | oxítona | `acentuar` |
| `-is`, `-us` (2+ sílabas, sem acento) | oxítona (verbo) | `eh_oxitona_is_us` |
| `-om`, `-um` (2+ sílabas, sem acento) | oxítona | `eh_oxitona_om_um` |
| `-irdes` | tônica no `i` | `eh_oxitona_irdes` |
| `-r`, `-l`, `-z`, `-x`, `-n` final | oxítona | `acentuar` |
| ditongo + `s` final | oxítona | `acentuar` |
| `-ai`, `-ei`, `-oi`, `-au`, `-eu`, `-iu`, `-ou` | oxítona | `acentuar` |
| `-i`, `-u` final (1 sílaba) | oxítona | `acentuar` |
| `-em`, `-ens`, `-am`, `-ams` | paroxítona forçada | `acentuar` |
| demais | paroxítona (padrão) | `acentuar` |

## 12. Acento secundário

| Condição | Marcas | Onde |
|---|---|---|
| palavra 3+ sílabas, tônica não-1ª, sem prefixo `sobre-` | ˌ nas sílabas 0, 2, 4... | `acento_secundario` |
| palavra com prefixo `sobre-` (4+ sílabas) | ˌ a partir da sílaba 2 | `acento_secundario` |
| palavra 2 sílabas | nenhuma | `acento_secundario` |

## 13. Sândi entre palavras

| Contexto | Efeito | Onde |
|---|---|---|
| `s` final + próxima palavra vogal/`b,d,g,j,l,m,n,r,v,z` | `z` | `sonorizar_s` |
| `s` final + próxima surda/pausa | `s` | `sonorizar_s` |
| `e` final antes de vogal | `y` (redução) | `vogal_oral` |

## 14. Pós-processamento (`limpar`)

| Regra | Antes | Depois |
|---|---|---|
| NFD | `ɐ̃` (U+00E3) | `ɐ` + `̃` (U+0303) |
| `g` ASCII | `g` | `ɡ` (U+0261) |
| acento duplicado | `ˈˈ` | `ˈ` |
| til duplicado | `̃̃` | `̃` |
| `ss` colapsa | `ss` | `s` |
| `zs` colapsa | `zs` | `s` |
| sufixo `-ídeo` | `ideʊ` | `idʒjʊ` |

## 15. Prefixos e sufixos especiais

| Prefixo/Sufixo | Efeito | Onde |
|---|---|---|
| `ex-` + vogal | `x` → `z` | `prefixo_ex` |
| `sobre-` (4+ sílabas) | sem ˌ na sílaba 0 | `tem_prefixo_sobre` |
| `-am` final | `ɐ̃ʊ̃` | `eh_am_final` |
| radicais `taxi-`,`fix-`,`sex-`,`toxic-`,`reflex-`,`anex-`,`flux-`,`nex-`,`paradox-`,`ortodox-`,`prolix-`,`axiom-`,`axil-`,`asfixi-`,`crucifix-`,`toxin-`,`elix-`,`climax-`,`hidrox-`,`carbox-`,`oxid-`,`oxig-`,`oxil-`,`dioxid-`,`peroxid-`,`superoxid-`,`hexa-`,`flex-` | `x` → `ks` | `RADICAIS_KS` |
| exceções `sext-`, `anexim` | `x` → `ʃ` | `EXCECOES_KS` |

## 16. Clíticos (do `lexicon.rs`, hardcoded)

| Classe | Palavras | IPA |
|---|---|---|
| Artigos | `a`, `as`, `o`, `os` | `a`, `as`, `ʊ`, `ʊs` |
| Indefinidos | `um`, `uns`, `uma`, `umas` | `ũŋ`, `ũŋs`, `umæ`, `umæs` |
| Preposições | `de`, `do`, `da`, `dos`, `das`, `em`, `no`, `na`, `nos`, `nas`, `num`, `numa`, `por`, `pelo`, `pela`, `pelos`, `pelas`, `ao`, `aos`, `à`, `às` | ... |
| Conjunções/pronomes | `e`, `ou`, `que`, `se`, `me`, `te`, `lhe`, `vos`, `com`, `sem`, `para`, `pra`, `é` | ... |

## 17. Léxico interno (`LEXICO` hardcoded)

Aproximadamente 150 entradas com palavras de alta frequência, vogal tônica imprevisível, dígrafos difíceis e estrangeirismos. Ver `src/lexicon.rs`.

## 18. Léxico externo (`lexico_espeak.json`)

Gerado pelo `compare_vozz.py`. Só entram palavras onde `rs != espeak`. Atualmente ~182k, mas deveria cair drasticamente com o fix do `gerar_lexico`.

---

## O que ainda **não** está mapeado (candidato a léxico ou regra)

| Caso | Exemplo | Status |
|---|---|---|
| `o` tônico + `r` + cons | `porta` (aberto) vs `forno` (fechado) | lexical |
| `e` tônico + `r` | `quer` (aberto) vs `adulter` (fechado) | lexical |
| `-el` final | `papel` → `pˈɛʊ` | pendente |
| `qu` + `e`/`i` no meio | `frequência` → `frˌekw...` | pendente |
| `iu` antes de consoante | `iurumi` → `jˌuɾumˈi` | pendente |
| `re` + `u`/`i` | `reunir` (hiato) vs `reusar` (ditongo) | lexical |
| `-l` final | `sol` → `sˈɔl`, `gol` → `ɡˈow` | lexical |
| Estrangeirismos | `playboy`, `marketing` | lexical |
| Siglas sem vogal | `ldl`, `ngf` | filtrado |

Esse mapa cobre tudo que o `g2p.rs` produz hoje. Se quiser, posso gerar uma versão em Markdown pronta para colocar no README do repo.
