//! Léxico de exceções pt-BR.
//!
//! Convenção IPA idêntica à do espeak-ng `pt-br`. Use `ɡ` (U+0261).
//! Strings em NFD.

use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Palavras átonas (clíticos): entram na cadeia sem acento primário.
pub static CLITICOS: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut mapa = HashMap::new();

    mapa.insert("a", "a");
    mapa.insert("as", "as");
    mapa.insert("o", "ʊ");
    mapa.insert("os", "ʊs");
    mapa.insert("um", "u\u{0303}ŋ");
    mapa.insert("uns", "u\u{0303}ŋs");
    mapa.insert("uma", "umæ");
    mapa.insert("umas", "umæs");
    mapa.insert("de", "dʒj");
    mapa.insert("do", "dʊ");
    mapa.insert("da", "da");
    mapa.insert("dos", "dʊs");
    mapa.insert("das", "das");
    // `em` NÃO é clítico: em contexto, o espeak mantém acento primário.
    // Fica no LEXICO.
    mapa.insert("no", "nʊ");
    mapa.insert("na", "na");
    mapa.insert("nos", "nʊs");
    mapa.insert("nas", "nas");
    mapa.insert("num", "nu\u{0303}ŋ");
    mapa.insert("numa", "numæ");
    mapa.insert("pelo", "pelʊ");
    mapa.insert("pela", "pelæ");
    mapa.insert("pelos", "pelʊs");
    mapa.insert("pelas", "pelæs");
    mapa.insert("ao", "aʊ");
    mapa.insert("aos", "aʊs");
    mapa.insert("à", "a");
    mapa.insert("às", "as");
    mapa.insert("e", "i");
    mapa.insert("ou", "oʊ");
    mapa.insert("que", "ky");
    mapa.insert("se", "sj");
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

/// Clíticos que mudam de pronúncia **em contexto de sentença**.
///
/// Verificado contra `espeak-ng`: quando isolados, essas palavras têm
/// acento primário; dentro de uma sentença, o espeak remove o acento
/// (átonos) ou rebaixa para secundário.
pub static CLITICOS_CONTEXTO: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut mapa = HashMap::new();

    // Átonos em contexto.
    mapa.insert("que", "ky");
    mapa.insert("na", "na");
    mapa.insert("nas", "nas");
    mapa.insert("no", "nʊ");
    mapa.insert("nos", "nʊs");
    mapa.insert("os", "ʊs");
    mapa.insert("as", "as");
    mapa.insert("de", "dʒy");
    mapa.insert("por", "poɾ");
    mapa.insert("com", "koŋ");
    mapa.insert("tem", "teɪŋ");
    mapa.insert("vai", "vaɪ");
    mapa.insert("ser", "seɾ");
    mapa.insert("ter", "teɾ");
    mapa.insert("se", "sj");
    mapa.insert("mas", "maz");
    mapa.insert("foi", "foɪ");
    mapa.insert("pelo", "pelʊ");

    // Secundários em contexto.
    mapa.insert("para", "pˌaɾæ");
    mapa.insert("onde", "ˌoŋdʒy");
    mapa.insert("pode", "pˌɔdʒy");
    mapa.insert("ele", "ˌely");
    mapa.insert("ela", "ˌɛlæ");
    mapa.insert("esse", "ˌesi");
    mapa.insert("este", "ˌestʃy");
    mapa.insert("esta", "ˌɛstæ");
    mapa.insert("estou", "estˌow");
    mapa.insert("está", "estˌa");
    mapa.insert("estão", "estˌɐ\u{0303}ʊ\u{0303}");
    mapa.insert("estar", "estˌaɾ");
    mapa.insert("você", "vosˌe");
    mapa.insert("vocês", "vosˌes");
    mapa.insert("uma", "ˌumæ");
    mapa.insert("à", "ˌaː");
    mapa.insert("às", "ˌaːs");

    // `fazer` mantém acento primário em contexto.
    mapa.insert("fazer", "fazˈer");

    mapa
});

/// Exceções lexicais plenas (com marca de tônica).
pub static LEXICO: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut mapa = HashMap::new();

    mapa.insert("não", "nˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("sim", "sˈiŋ");
    mapa.insert("também", "tɐ\u{0303}mbˈeɪŋ");
    mapa.insert("então", "eɪŋtˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("muito", "mwˈiŋtʊ");
    mapa.insert("muita", "mwˈiŋtæ");
    mapa.insert("muitos", "mwˈiŋtʊs");
    mapa.insert("muitas", "mwˈiŋtæs");
    mapa.insert("bem", "bˈeɪŋ");
    mapa.insert("têm", "tˈeɪŋ");
    mapa.insert("vem", "vˈeɪŋ");
    mapa.insert("em", "ˈeɪŋ");
    mapa.insert("são", "sˈɐ\u{0303}ʊ\u{0303}");
    mapa.insert("ver", "vˈer");
    mapa.insert("vir", "vˈir");
    mapa.insert("pôr", "pˈor");
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
    mapa.insert("até", "atˈɛ");
    mapa.insert("após", "apˈɔs");
    mapa.insert("mais", "mˈaɪs");
    mapa.insert("menos", "mˈenʊs");
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
    mapa.insert("eles", "ˈelys");
    mapa.insert("elas", "ˈɛlæs");
    mapa.insert("eu", "ˈeʊ");
    mapa.insert("tu", "tˈu");
    mapa.insert("nós", "nˈɔs");
    mapa.insert("vós", "vˈɔs");
    mapa.insert("gente", "ʒˈeɪŋtʃy");
    mapa.insert("isso", "ˈisʊ");
    mapa.insert("isto", "ˈistʊ");
    mapa.insert("aquilo", "ˌakˈilʊ");
    mapa.insert("essa", "ˈɛsæ");
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
    mapa.insert("novos", "nˈovʊs");
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

/// Consulta o léxico.
pub fn buscar_lexico(palavra: &str) -> Option<&'static str> {
    let chave = palavra.to_lowercase();
    LEXICO.get(chave.as_str()).copied()
}

/// Consulta clíticos átonos.
pub fn buscar_clitico(palavra: &str) -> Option<&'static str> {
    let chave = palavra.to_lowercase();
    CLITICOS.get(chave.as_str()).copied()
}

/// Consulta clíticos em contexto de sentença.
pub fn buscar_clitico_contexto(palavra: &str) -> Option<&'static str> {
    let chave = palavra.to_lowercase();
    CLITICOS_CONTEXTO.get(chave.as_str()).copied()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn lexicon_encontra_palavras_comuns() {
        assert!(buscar_lexico("não").is_some());
        assert!(buscar_lexico("também").is_some());
        assert!(buscar_lexico("exemplo").is_some());
        assert!(buscar_lexico("software").is_some());
        assert!(buscar_lexico("em").is_some());
        assert!(buscar_lexico("casa").is_none());
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
    fn cliticos_contexto_encontram_palavras() {
        assert!(buscar_clitico_contexto("que").is_some());
        assert!(buscar_clitico_contexto("na").is_some());
        assert!(buscar_clitico_contexto("para").is_some());
        assert!(buscar_clitico_contexto("onde").is_some());
        assert!(buscar_clitico_contexto("ser").is_some());
        assert!(buscar_clitico_contexto("você").is_some());
        assert!(buscar_clitico_contexto("os").is_some());
        assert!(buscar_clitico_contexto("as").is_some());
        assert!(buscar_clitico_contexto("por").is_some());
        assert!(buscar_clitico_contexto("com").is_some());
    }

    #[test]
    fn cliticos_contexto_de_correto() {
        // `de` em contexto → `dʒy`.
        assert_eq!(buscar_clitico_contexto("de"), Some("dʒy"));
    }

    #[test]
    fn clitico_de_fora_de_contexto_e_dʒj() {
        // `de` isolado → `dʒj`.
        assert_eq!(buscar_clitico("de"), Some("dʒj"));
    }

    #[test]
    fn cliticos_contexto_uma_secundario() {
        assert_eq!(buscar_clitico_contexto("uma"), Some("ˌumæ"));
    }

    #[test]
    fn cliticos_contexto_case_insensitive() {
        assert_eq!(buscar_clitico_contexto("Que"), buscar_clitico_contexto("que"));
        assert_eq!(buscar_clitico_contexto("PARA"), buscar_clitico_contexto("para"));
        assert_eq!(buscar_clitico_contexto("Os"), buscar_clitico_contexto("os"));
    }

    #[test]
    fn busca_e_case_insensitive() {
        assert_eq!(buscar_lexico("NÃO"), buscar_lexico("não"));
        assert_eq!(buscar_lexico("Não"), buscar_lexico("não"));
        assert_eq!(buscar_clitico("DE"), buscar_clitico("de"));
    }

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
    }

    #[test]
    fn sem_g_ascii_nos_valores() {
        for (chave, valor) in LEXICO.iter() {
            assert!(
                !valor.contains('\u{0067}'),
                "valor de {:?} contém g ASCII: {:?}",
                chave, valor
            );
        }
        for (chave, valor) in CLITICOS.iter() {
            assert!(!valor.contains('\u{0067}'), "clítico {:?}: {:?}", chave, valor);
        }
    }

    #[test]
    fn nao_ha_chaves_com_underscore() {
        assert!(buscar_clitico("nos_").is_none());
        assert!(buscar_clitico("às_").is_none());
        assert!(buscar_clitico("nos").is_some());
        assert!(buscar_clitico("às").is_some());
    }
}
