//! Léxico de exceções pt-BR.
//!
//! Porte de `lexicon.js` do Vozz.
//!
//! Cobre:
//! - palavras de altíssima frequência;
//! - casos em que a vogal tônica aberta/fechada não é dedutível pela ortografia;
//! - estrangeirismos.
//!
//! Convenção IPA idêntica à do espeak-ng `pt-br`. Use `ɡ` (U+0261), nunca
//! o `g` ASCII: o `g` latino não existe no vocabulário do modelo.
//!
//! Todas as strings estão em NFD (vogal + til combinante U+0303). Sempre
//! escrevemos o til como `\u{0303}` para que nenhum editor precomponha.
//!
//! As chaves `nos_` e `às_` do léxico JS foram descartadas por serem
//! duplicatas com underscore que nunca casam com texto real.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Palavras átonas (clíticos): entram na cadeia sem acento primário.
pub static CLITICOS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut mapa = HashMap::new();

    // Artigos definidos e indefinidos.
    mapa.insert("a", "a");
    mapa.insert("as", "as");
    mapa.insert("o", "ʊ");
    mapa.insert("os", "ʊs");
    mapa.insert("um", "u\u{0303}ŋ");
    mapa.insert("uns", "u\u{0303}ŋs");
    mapa.insert("uma", "umæ");
    mapa.insert("umas", "umæs");

    // Preposições e contrações.
    mapa.insert("de", "dʒy");
    mapa.insert("do", "dʊ");
    mapa.insert("da", "da");
    mapa.insert("dos", "dʊs");
    mapa.insert("das", "das");
    mapa.insert("em", "eɪŋ");
    mapa.insert("no", "nʊ");
    mapa.insert("na", "na");
    mapa.insert("nos", "nʊs");
    mapa.insert("nas", "nas");
    mapa.insert("num", "nu\u{0303}ŋ");
    mapa.insert("numa", "numæ");
    mapa.insert("por", "por");
    mapa.insert("pelo", "pelʊ");
    mapa.insert("pela", "pelæ");
    mapa.insert("pelos", "pelʊs");
    mapa.insert("pelas", "pelæs");
    mapa.insert("ao", "aʊ");
    mapa.insert("aos", "aʊs");
    mapa.insert("à", "a");
    mapa.insert("às", "as");

    // Conjunções, pronomes, advérbios átonos.
    mapa.insert("e", "i");
    mapa.insert("ou", "oʊ");
    mapa.insert("que", "ky");
    mapa.insert("se", "sy");
    mapa.insert("me", "my");
    mapa.insert("te", "tʃy");
    mapa.insert("lhe", "ʎy");
    mapa.insert("vos", "vus");
    mapa.insert("com", "koŋ");
    mapa.insert("sem", "seɪŋ");
    mapa.insert("para", "paɾæ");
    mapa.insert("pra", "pra");
    mapa.insert("é", "ɛ");

    mapa
});

/// Exceções lexicais plenas (com marca de tônica).
/// Verificadas contra `espeak-ng -v pt-br -q --ipa`.
pub static LEXICO: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut mapa = HashMap::new();

    // --- verbos e palavras funcionais de alta frequência ---
    mapa.insert("não", "nˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("sim", "sˈiŋ");
    mapa.insert("também", "tɐ\u{0303}mbˈeɪŋ");
    mapa.insert("então", "eɪŋtˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("muito", "mwˈiŋtʊ");
    mapa.insert("muita", "mwˈiŋtæ");
    mapa.insert("muitos", "mwˈiŋtʊs");
    mapa.insert("muitas", "mwˈiŋtæs");
    mapa.insert("bem", "bˈeɪŋ");
    mapa.insert("tem", "tˈeɪŋ");
    mapa.insert("têm", "tˈeɪŋ");
    mapa.insert("vem", "vˈeɪŋ");
    mapa.insert("são", "sˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("estão", "estˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("está", "estˈa");
    mapa.insert("estou", "estˈow");
    mapa.insert("ser", "sˈer");
    mapa.insert("ter", "tˈer");
    mapa.insert("ver", "vˈer");
    mapa.insert("vir", "vˈir");
    mapa.insert("pôr", "pˈor");
    mapa.insert("fazer", "fazˈer");
    mapa.insert("dizer", "dʒizˈer");
    mapa.insert("poder", "podˈer");
    mapa.insert("querer", "keɾˈer");
    mapa.insert("hoje", "ˈoʒy");
    mapa.insert("ontem", "ˈoŋteɪŋ");
    mapa.insert("amanhã", "ˌæmɐ\u{0303}ɲˈɐ\u{0303}");
    mapa.insert("agora", "ˌaɡˈɔɾæ");
    mapa.insert("depois", "depˈoɪs");
    mapa.insert("antes", "ˈɐ\u{0303}ŋtʃys");
    mapa.insert("sempre", "sˈeɪmpry");
    mapa.insert("nunca", "nˈu\u{0303}ŋkæ");
    mapa.insert("ainda", "ˌaˈiŋdæ");
    mapa.insert("aqui", "akˈi");
    mapa.insert("ali", "alˈi");
    mapa.insert("lá", "lˈa");
    mapa.insert("cá", "kˈa");
    mapa.insert("já", "ʒˈa");
    mapa.insert("só", "sˈɔ");
    mapa.insert("até", "ɛaɡudʊ");
    mapa.insert("após", "apˈɔs");
    mapa.insert("mais", "mˈaɪs");
    mapa.insert("mas", "mˈas");
    mapa.insert("menos", "mˈenʊs");
    mapa.insert("onde", "ˈoŋdʒy");
    mapa.insert("quando", "kwˈɐ\u{0303}ŋdʊ");
    mapa.insert("como", "kˈomʊ");
    mapa.insert("porque", "pˈoɾəky");
    mapa.insert("porquê", "poɾəkˈe");
    mapa.insert("quem", "kˈeɪŋ");
    mapa.insert("qual", "kwˈaʊ");
    mapa.insert("quais", "kwˈaɪs");
    mapa.insert("quanto", "kwˈɐ\u{0303}ŋtʊ");
    mapa.insert("todo", "tˈodʊ");
    mapa.insert("toda", "tˈodæ");
    mapa.insert("todos", "tˈodʊs");
    mapa.insert("todas", "tˈodæs");
    mapa.insert("outro", "ˈowtrʊ");
    mapa.insert("outra", "ˈowtræ");
    mapa.insert("mesmo", "mˈezmʊ");
    mapa.insert("mesma", "mˈezmæ");
    mapa.insert("cada", "kˈadæ");
    mapa.insert("algum", "aʊɡˈu\u{0303}ŋ");
    mapa.insert("alguma", "ˌaʊɡˈumæ");
    mapa.insert("nada", "nˈadæ");
    mapa.insert("tudo", "tˈudʊ");
    mapa.insert("algo", "ˈaʊɡʊ");
    mapa.insert("ele", "ˈely");
    mapa.insert("ela", "ˈɛlæ");
    mapa.insert("eles", "ˈelys");
    mapa.insert("elas", "ˈɛlæs");
    mapa.insert("eu", "ˈeʊ");
    mapa.insert("tu", "tˈu");
    mapa.insert("nós", "nˈɔs");
    mapa.insert("vós", "vˈɔs");
    mapa.insert("você", "vosˈe");
    mapa.insert("vocês", "vosˈes");
    mapa.insert("gente", "ʒˈeɪŋtʃy");
    mapa.insert("isso", "ˈisʊ");
    mapa.insert("isto", "ˈistʊ");
    mapa.insert("aquilo", "ˌakˈilʊ");
    mapa.insert("esse", "ˈesi");
    mapa.insert("essa", "ˈɛsæ");
    mapa.insert("este", "ˈestʃy");
    mapa.insert("esta", "ˈɛstæ");
    mapa.insert("meu", "mˈeʊ");
    mapa.insert("minha", "mˈiɲæ");
    mapa.insert("seu", "sˈeʊ");
    mapa.insert("sua", "sˈuæ");
    mapa.insert("nosso", "nˈɔsʊ");
    mapa.insert("nossa", "nˈɔsæ");
    mapa.insert("obrigado", "ˌobriɡˈadʊ");
    mapa.insert("obrigada", "ˌobriɡˈadæ");
    mapa.insert("favor", "favˈor");
    mapa.insert("desculpa", "dˌeskˈuwpæ");
    mapa.insert("oi", "ˈoɪ");
    mapa.insert("olá", "olˈa");
    mapa.insert("tchau", "tʃˈaʊ");
    mapa.insert("bom", "bˈoŋ");
    mapa.insert("boa", "bˈoæ");
    mapa.insert("boas", "bˈoæs");
    mapa.insert("bons", "bˈoŋs");
    mapa.insert("dia", "dʒˈiæ");
    mapa.insert("dias", "dʒˈiæs");
    mapa.insert("noite", "nˈoɪtʃy");
    mapa.insert("tarde", "tˈaɾədʒy");
    mapa.insert("ano", "ˈɐ\u{0303}nʊ");
    mapa.insert("anos", "ˈɐ\u{0303}nʊs");
    mapa.insert("mês", "mˈes");
    mapa.insert("hora", "ˈɔɾæ");
    mapa.insert("horas", "ˈɔɾæs");
    mapa.insert("vez", "vˈes");
    mapa.insert("vezes", "vˈezys");
    mapa.insert("coisa", "kˈoɪzæ");
    mapa.insert("coisas", "kˈoɪzæs");
    mapa.insert("pessoa", "pˌesˈoæ");
    mapa.insert("pessoas", "pˌesˈoæs");
    mapa.insert("brasil", "brazˈiʊ");
    mapa.insert("brasileiro", "brˌazilˈeɪɾʊ");
    mapa.insert("brasileira", "brˌazilˈeɪɾæ");
    mapa.insert("português", "pˌoɾətuɡˈes");
    mapa.insert("portuguesa", "pˌoɾətuɡˈezæ");
    mapa.insert("senhor", "seɲˈor");
    mapa.insert("senhora", "sˌeɲˈɔɾæ");

    // --- vogais tônicas abertas imprevisíveis ---
    mapa.insert("café", "kafˈɛ");
    mapa.insert("pé", "pˈɛ");
    mapa.insert("fé", "fˈɛ");
    mapa.insert("né", "nˈɛ");
    mapa.insert("avó", "avˈɔ");
    mapa.insert("avô", "avˈo");
    mapa.insert("vovó", "vovˈɔ");
    mapa.insert("vovô", "vovˈo");
    mapa.insert("história", "ˌistˈɔɾjæ");
    mapa.insert("memória", "mˌemˈɔɾjæ");
    mapa.insert("vitória", "vˌitˈɔɾjæ");
    mapa.insert("possível", "pˌosˈiveʊ");
    mapa.insert("difícil", "dʒˌifˈisiʊ");
    mapa.insert("fácil", "fˈasiʊ");
    mapa.insert("útil", "ˈutʃiʊ");
    mapa.insert("nível", "nˈiveʊ");
    mapa.insert("móvel", "mˈɔvɛʊ");
    mapa.insert("novo", "nˈovʊ");
    mapa.insert("nova", "nˈɔvæ");
    mapa.insert("novos", "nˈɔvʊs");
    mapa.insert("novas", "nˈɔvæs");
    mapa.insert("jogo", "ʒˈoɡʊ");
    mapa.insert("jogos", "ʒˈɔɡʊs");
    mapa.insert("porco", "pˈoɾəkʊ");
    mapa.insert("corpo", "kˈoɾəpʊ");
    mapa.insert("força", "fˈoɾəsæ");
    mapa.insert("morte", "mˈɔɾətʃy");
    mapa.insert("sorte", "sˈɔɾətʃy");
    mapa.insert("sol", "sˈɔl");
    mapa.insert("mar", "mˈar");
    mapa.insert("flor", "flˈor");
    mapa.insert("cor", "kˈor");
    mapa.insert("melhor", "meljˈɔr");
    mapa.insert("pior", "piˈɔr");
    mapa.insert("maior", "maɪˈɔr");
    mapa.insert("amor", "æmˈor");
    mapa.insert("dor", "dˈor");
    mapa.insert("calor", "kalˈor");
    mapa.insert("papel", "papˈɛʊ");
    mapa.insert("anel", "ɐ\u{0303}nˈɛʊ");
    mapa.insert("hotel", "otˈɛʊ");
    mapa.insert("água", "ˈaɡwæ");
    mapa.insert("língua", "lˈiŋɡwæ");
    mapa.insert("antigo", "ˌɐ\u{0303}ŋtʃˈiɡʊ");

    // --- dígrafos e grupos difíceis ---
    mapa.insert("exemplo", "ˌezˈeɪmplʊ");
    mapa.insert("exato", "ˌezˈatʊ");
    mapa.insert("exame", "ˌezˈɐ\u{0303}my");
    mapa.insert("texto", "tˈestʊ");
    mapa.insert("próximo", "prˈɔsimʊ");
    mapa.insert("máximo", "mˈasimʊ");
    mapa.insert("sexta", "sˈestæ");
    mapa.insert("excelente", "ˌeselˈeɪŋtʃy");
    mapa.insert("exercício", "ˌezeɾəsˈisjʊ");
    mapa.insert("táxi", "tˈaksi");
    mapa.insert("fixo", "fˈiksʊ");
    mapa.insert("sexo", "sˈɛksʊ");
    mapa.insert("peixe", "pˈeɪʃy");
    mapa.insert("caixa", "kˈaɪʃæ");
    mapa.insert("baixo", "bˈaɪʃʊ");
    mapa.insert("trabalho", "trˌabˈaljʊ");
    mapa.insert("filho", "fˈiljʊ");
    mapa.insert("filha", "fˈiljæ");
    mapa.insert("velho", "vˈɛljʊ");
    mapa.insert("olho", "ˈɔljʊ");
    mapa.insert("milho", "mˈiljʊ");
    mapa.insert("vinho", "vˈiɲʊ");
    mapa.insert("sonho", "sˈoɲʊ");
    mapa.insert("banho", "bˈɐ\u{0303}ɲʊ");
    mapa.insert("tenho", "tˈeɲʊ");
    mapa.insert("venho", "vˈeɲʊ");
    mapa.insert("ganho", "ɡˈɐ\u{0303}ɲʊ");
    mapa.insert("carro", "kˈaxʊ");
    mapa.insert("caro", "kˈaɾʊ");
    mapa.insert("terra", "tˈɛxæ");
    mapa.insert("cara", "kˈaɾæ");
    mapa.insert("guerra", "ɡˈɛxæ");
    mapa.insert("cachorro", "kˌaʃˈoxʊ");
    mapa.insert("correr", "koxˈer");
    mapa.insert("razão", "xazˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("coração", "kˌoɾasˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("nação", "nasˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("mãe", "mˈɐ\u{0303}y");
    mapa.insert("pão", "pˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("cão", "kˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("irmão", "iɾəmˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("alemão", "ˌalemˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("verão", "veɾˈɐ\u{0303}ʊ\u{0303}");

    // --- tecnologia / estrangeirismos ---
    mapa.insert("site", "sˈaɪtʃy");
    mapa.insert("email", "ˌemaˈiʊ");
    mapa.insert("online", "oŋlˈaɪŋ");
    mapa.insert("software", "sˈɔftweə");
    mapa.insert("hardware", "xˈaɾdiwɛɾ");
    mapa.insert("mouse", "mˈaʊzi");
    mapa.insert("internet", "ˌiŋteɾənˈɛtʃ");
    mapa.insert("wifi", "wifˈi");
    mapa.insert("app", "ˈap");
    mapa.insert("npm", "ˈeni pˈe ˈemi");
    mapa.insert("api", "ˌapiˈi");
    mapa.insert("url", "ˌuˈɛli");
    mapa.insert("css", "sˈe ˈesi ˈesi");
    mapa.insert("html", "ˌaɡˈa tˈe ˈemi ˈɛli");
    mapa.insert("sql", "ˌɛsi kˈu ˈɛli");
    mapa.insert("cli", "sˈe ˈɛli ˈi");
    mapa.insert("json", "ʒˈejzo\u{0303}ŋ");
    mapa.insert("npx", "ˈeni pˈe ʃˈis");
    mapa.insert("smartphone", "zmˌaɾətfˈony");
    mapa.insert("link", "lˈiŋk");
    mapa.insert("download", "daʊŋlˈowd");
    mapa.insert("design", "dezˈaɪn");
    mapa.insert("startup", "staɾətˈup");
    mapa.insert("feedback", "fˌeedbˈak");
    mapa.insert("google", "ɡˈuɡol");
    mapa.insert("youtube", "jˌowtˈuby");
    mapa.insert("whatsapp", "watsˈap");

    mapa
});

/// Consulta o léxico. Devolve `None` se a palavra não estiver mapeada.
pub fn buscar_lexico(palavra: &str) -> Option<&'static str> {
    let chave = palavra.to_lowercase();
    LEXICO.get(chave.as_str()).copied()
}

/// Consulta clíticos átonos. Devolve `None` se não estiver mapeado.
pub fn buscar_clitico(palavra: &str) -> Option<&'static str> {
    let chave = palavra.to_lowercase();
    CLITICOS.get(chave.as_str()).copied()
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn lexicon_encontra_palavras_comuns() {
        assert!(buscar_lexico("não").is_some());
        assert!(buscar_lexico("também").is_some());
        assert!(buscar_lexico("você").is_some());
        assert!(buscar_lexico("exemplo").is_some());
        assert!(buscar_lexico("software").is_some());
    }

    #[test]
    fn lexicon_palavra_ausente_retorna_none() {
        assert!(buscar_lexico("xyzabc").is_none());
        assert!(buscar_lexico("palavraqualquer").is_none());
        assert!(buscar_lexico("").is_none());
    }

    #[test]
    fn cliticos_encontram_palavras_comuns() {
        assert!(buscar_clitico("de").is_some());
        assert!(buscar_clitico("para").is_some());
        assert!(buscar_clitico("com").is_some());
        assert!(buscar_clitico("que").is_some());
    }

    #[test]
    fn cliticos_palavra_ausente_retorna_none() {
        assert!(buscar_clitico("casa").is_none());
        assert!(buscar_clitico("").is_none());
    }

    #[test]
    fn busca_e_case_insensitive() {
        assert_eq!(buscar_lexico("NÃO"), buscar_lexico("não"));
        assert_eq!(buscar_lexico("Não"), buscar_lexico("não"));
        assert_eq!(buscar_clitico("DE"), buscar_clitico("de"));
    }

    /// O léxico precisa estar inteiramente em NFD.
    ///
    /// O modelo do Piper foi treinado com o vocabulário do espeak-ng, que usa
    /// vogal + til combinante (U+0303), nunca a forma precomposta. Se um valor
    /// escapar como precomposto, o tokenizer do Piper descarta silenciosamente
    /// e o som sai errado sem erro.
    ///
    /// Este teste cobre todas as formas precompostas que poderiam aparecer
    /// (til, agudo, circunflexo, grave) tanto no léxico quanto nos clíticos.
    #[test]
    fn valores_estao_em_nfd() {
        let precompostos: [(char, &str); 12] = [
            ('\u{00E3}', "ã"),
            ('\u{00F5}', "õ"),
            ('\u{0169}', "ũ"),
            ('\u{00E1}', "á"),
            ('\u{00E9}', "é"),
            ('\u{00ED}', "í"),
            ('\u{00F3}', "ó"),
            ('\u{00FA}', "ú"),
            ('\u{00E2}', "â"),
            ('\u{00EA}', "ê"),
            ('\u{00F4}', "ô"),
            ('\u{00E0}', "à"),
        ];

        for (chave, valor) in LEXICO.iter() {
            for &(caractere, nome) in &precompostos {
                assert!(
                    !valor.contains(caractere),
                    "valor de {:?} contém '{}' precomposto: {:?}",
                    chave, nome, valor
                );
            }
        }
        for (chave, valor) in CLITICOS.iter() {
            for &(caractere, nome) in &precompostos {
                assert!(
                    !valor.contains(caractere),
                    "clítico {:?} contém '{}' precomposto: {:?}",
                    chave, nome, valor
                );
            }
        }

        // Verificação positiva: "não" precisa conter til combinante.
        let nao = buscar_lexico("não").unwrap();
        assert!(
            nao.contains('\u{0303}'),
            "esperava til combinante em 'não', mas veio: {:?}",
            nao
        );
    }

    /// O léxico não pode conter o `g` ASCII (U+0067).
    ///
    /// Existem dois caracteres visualmente idênticos em Unicode:
    ///   - `g` LATIN SMALL LETTER G (U+0067)
    ///   - `ɡ` LATIN SMALL LETTER SCRIPT G (U+0261)
    ///
    /// O vocabulário do Piper usa exclusivamente U+0261 para o fonema /ɡ/.
    /// Se um U+0067 escapar, o tokenizer do Piper não o encontra no vocabulário
    /// e o som é descartado silenciosamente — a palavra sai muda nesse trecho.
    ///
    /// Este teste percorre todos os valores do léxico e dos clíticos para
    /// garantir que nenhum contém U+0067.
    #[test]
    fn sem_g_ascii_nos_valores() {
        for (chave, valor) in LEXICO.iter() {
            assert!(
                !valor.contains('\u{0067}'),
                "valor de {:?} contém g ASCII (U+0067): {:?}",
                chave, valor
            );
        }
        for (chave, valor) in CLITICOS.iter() {
            assert!(
                !valor.contains('\u{0067}'),
                "clítico {:?} contém g ASCII (U+0067): {:?}",
                chave, valor
            );
        }
    }

    #[test]
    fn nao_ha_chaves_com_underscore() {
        // As duplicatas `nos_` e `às_` do léxico JS foram descartadas.
        assert!(buscar_clitico("nos_").is_none());
        assert!(buscar_clitico("às_").is_none());
        // Mas as versões sem underscore existem.
        assert!(buscar_clitico("nos").is_some());
        assert!(buscar_clitico("às").is_some());
    }
}