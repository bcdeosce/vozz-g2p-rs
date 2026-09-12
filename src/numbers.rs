//! Extenso de números em português do Brasil.
//! Porte fiel de `numbers.js` do Vozz, com correções de bugs conhecidos.

const UNIDADES: [&str; 10] = [
    "zero", "um", "dois", "três", "quatro",
    "cinco", "seis", "sete", "oito", "nove",
];

const DEZ_A_DEZENOVE: [&str; 10] = [
    "dez", "onze", "doze", "treze", "quatorze",
    "quinze", "dezesseis", "dezessete", "dezoito", "dezenove",
];

const DEZENAS: [&str; 10] = [
    "", "", "vinte", "trinta", "quarenta",
    "cinquenta", "sessenta", "setenta", "oitenta", "noventa",
];

const CENTENAS: [&str; 10] = [
    "", "cento", "duzentos", "trezentos", "quatrocentos",
    "quinhentos", "seiscentos", "setecentos", "oitocentos", "novecentos",
];

const ESCALAS: [(&str, &str); 6] = [
    ("", ""),
    ("mil", "mil"),
    ("milhão", "milhões"),
    ("bilhão", "bilhões"),
    ("trilhão", "trilhões"),
    ("quatrilhão", "quatrilhões"),
];

const ORDINAIS_UNIDADES: [&str; 10] = [
    "", "primeiro", "segundo", "terceiro", "quarto",
    "quinto", "sexto", "sétimo", "oitavo", "nono",
];

const ORDINAIS_DEZENAS: [&str; 10] = [
    "", "décimo", "vigésimo", "trigésimo", "quadragésimo",
    "quinquagésimo", "sexagésimo", "septuagésimo", "octogésimo", "nonagésimo",
];

const ORDINAIS_CENTENAS: [&str; 10] = [
    "", "centésimo", "ducentésimo", "trecentésimo", "quadringentésimo",
    "quinquagésimo", "seiscentésimo", "septingentésimo", "octingentésimo", "noningentésimo",
];

/// Opções para a geração de extenso.
#[derive(Clone, Copy, Default)]
pub struct OpcoesExtenso {
    /// Se verdadeiro, gera formas femininas ("uma", "duas", "primeira").
    pub feminino: bool,
}

/// Converte uma palavra de unidade para sua forma feminina, se aplicável.
fn femininizar(palavra: &str) -> &str {
    match palavra {
        "um" => "uma",
        "dois" => "duas",
        _ => palavra,
    }
}

/// Converte um número entre 0 e 999 por extenso.
fn converter_ate_999(numero: u32, feminino: bool) -> String {
    if numero == 0 {
        return String::new();
    }
    if numero == 100 {
        return "cem".to_string();
    }

    let centena = (numero / 100) as usize;
    let resto = numero % 100;
    let mut partes: Vec<String> = Vec::new();

    if centena > 0 {
        let mut texto_centena = CENTENAS[centena].to_string();
        if feminino && centena >= 2 {
            // Todas as centenas de 200 a 900 terminam em "tos".
            if let Some(prefixo) = texto_centena.strip_suffix("tos") {
                texto_centena = format!("{}tas", prefixo);
            }
        }
        partes.push(texto_centena);
    }

    if resto > 0 {
        if resto < 10 {
            let unidade = resto as usize;
            let palavra = if feminino {
                femininizar(UNIDADES[unidade])
            } else {
                UNIDADES[unidade]
            };
            partes.push(palavra.to_string());
        } else if resto < 20 {
            partes.push(DEZ_A_DEZENOVE[(resto - 10) as usize].to_string());
        } else {
            let dezena = (resto / 10) as usize;
            let unidade = (resto % 10) as usize;
            if unidade == 0 {
                partes.push(DEZENAS[dezena].to_string());
            } else {
                let palavra_unidade = if feminino {
                    femininizar(UNIDADES[unidade])
                } else {
                    UNIDADES[unidade]
                };
                partes.push(format!("{} e {}", DEZENAS[dezena], palavra_unidade));
            }
        }
    }

    partes.join(" e ")
}

/// Junta os trechos de escalas diferentes usando "e" quando a norma pede.
///
/// Correção em relação ao Vozz original: entre grupos de escala diferentes
/// usa-se espaço, não vírgula. O original produzia
/// "mil, novecentos e noventa e oito"; o correto é
/// "mil novecentos e noventa e oito".
fn juntar_trechos(trechos: &[String], ultimo_grupo: u32) -> String {
    if trechos.is_empty() {
        return String::new();
    }
    if trechos.len() == 1 {
        return trechos[0].clone();
    }

    let total = trechos.len();
    let cabeca = trechos[..total - 1].join(" ");
    let cauda = &trechos[total - 1];

    if ultimo_grupo > 0 && (ultimo_grupo < 100 || ultimo_grupo % 100 == 0) {
        format!("{} e {}", cabeca, cauda)
    } else {
        format!("{} {}", cabeca, cauda)
    }
}

/// Converte um número inteiro por extenso.
pub fn inteiro_por_extenso(valor: i64, opcoes: OpcoesExtenso) -> String {
    let negativo = valor < 0;
    let mut restante: u64 = valor.unsigned_abs();

    if restante == 0 {
        return if negativo {
            "menos zero".to_string()
        } else {
            "zero".to_string()
        };
    }

    // Quebra o número em grupos de três dígitos (do menos significativo
    // para o mais significativo).
    let mut grupos: Vec<u32> = Vec::new();
    while restante > 0 {
        grupos.push((restante % 1000) as u32);
        restante /= 1000;
    }

    // Se passar da maior escala conhecida, soletra dígito a dígito.
    if grupos.len() > ESCALAS.len() {
        return soletrar_digitos(&valor.to_string());
    }

    let mut trechos: Vec<String> = Vec::new();
    for indice in (0..grupos.len()).rev() {
        let grupo = grupos[indice];
        if grupo == 0 {
            continue;
        }

        if indice == 1 {
            // "mil" nunca leva "um" antes.
            if grupo == 1 {
                trechos.push("mil".to_string());
            } else {
                trechos.push(format!("{} mil", converter_ate_999(grupo, opcoes.feminino)));
            }
        } else if indice == 0 {
            trechos.push(converter_ate_999(grupo, opcoes.feminino));
        } else {
            let (singular, plural) = ESCALAS[indice];
            let escala = if grupo == 1 { singular } else { plural };
            trechos.push(format!("{} {}", converter_ate_999(grupo, false), escala));
        }
    }

    let mut texto = juntar_trechos(&trechos, grupos[0]);
    if negativo {
        texto = format!("menos {}", texto);
    }
    texto
}

/// Converte um número decimal por extenso, dadas a parte inteira e a
/// parte fracionária como string.
pub fn decimal_por_extenso(inteiro: i64, fracao: &str, opcoes: OpcoesExtenso) -> String {
    let parte_inteira = inteiro_por_extenso(inteiro, opcoes);
    if fracao.is_empty() {
        return parte_inteira;
    }

    let sem_zeros_finais = fracao.trim_end_matches('0');
    if sem_zeros_finais.is_empty() {
        // Correção do bug do Vozz: parte fracionária zero → só a parte inteira.
        return parte_inteira;
    }
    let fracao_limpa = sem_zeros_finais;

    let quantidade_zeros_iniciais = fracao_limpa.chars().take_while(|caractere| *caractere == '0').count();
    let resto = &fracao_limpa[quantidade_zeros_iniciais..];

    let mut palavras: Vec<String> = Vec::with_capacity(quantidade_zeros_iniciais + 1);
    for _ in 0..quantidade_zeros_iniciais {
        palavras.push("zero".to_string());
    }
    if !resto.is_empty() {
        if let Ok(numero) = resto.parse::<i64>() {
            palavras.push(inteiro_por_extenso(numero, opcoes));
        }
    }

    format!("{} vírgula {}", parte_inteira, palavras.join(" "))
}

/// Converte um número ordinal por extenso.
pub fn ordinal_por_extenso(valor: i64, opcoes: OpcoesExtenso) -> String {
    if valor <= 0 || valor > 999 {
        return inteiro_por_extenso(valor, opcoes);
    }

    let numero = valor as u32;
    let centena = (numero / 100) as usize;
    let dezena = ((numero % 100) / 10) as usize;
    let unidade = (numero % 10) as usize;

    let mut partes: Vec<&str> = Vec::with_capacity(3);
    if centena > 0 {
        partes.push(ORDINAIS_CENTENAS[centena]);
    }
    if dezena > 0 {
        partes.push(ORDINAIS_DEZENAS[dezena]);
    }
    if unidade > 0 {
        partes.push(ORDINAIS_UNIDADES[unidade]);
    }

    let texto = partes.join(" ");

    if !opcoes.feminino {
        return texto;
    }

    // Substitui o 'o' final de cada palavra por 'a'.
    texto
        .split_whitespace()
        .map(|palavra| {
            if let Some(prefixo) = palavra.strip_suffix('o') {
                format!("{}a", prefixo)
            } else {
                palavra.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Soletra cada dígito de uma string, mantendo caracteres não numéricos
/// (exceto hífen e espaços em branco, que são descartados).
pub fn soletrar_digitos(entrada: &str) -> String {
    let mut saida = String::new();
    let mut primeiro = true;

    for caractere in entrada.chars() {
        let token: Option<String> = if caractere.is_ascii_digit() {
            let indice = (caractere as u8 - b'0') as usize;
            Some(UNIDADES[indice].to_string())
        } else if caractere == '-' || caractere.is_whitespace() {
            None
        } else {
            Some(caractere.to_string())
        };

        if let Some(texto) = token {
            if !primeiro {
                saida.push(' ');
            }
            saida.push_str(&texto);
            primeiro = false;
        }
    }

    saida
}

/// Converte um ano por extenso. Equivalente a `inteiro_por_extenso`.
pub fn ano_por_extenso(valor: i64) -> String {
    inteiro_por_extenso(valor, OpcoesExtenso::default())
}

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn unidades_e_dezenas() {
        let opcoes = OpcoesExtenso::default();
        assert_eq!(inteiro_por_extenso(0, opcoes), "zero");
        assert_eq!(inteiro_por_extenso(1, opcoes), "um");
        assert_eq!(inteiro_por_extenso(9, opcoes), "nove");
        assert_eq!(inteiro_por_extenso(10, opcoes), "dez");
        assert_eq!(inteiro_por_extenso(15, opcoes), "quinze");
        assert_eq!(inteiro_por_extenso(20, opcoes), "vinte");
        assert_eq!(inteiro_por_extenso(21, opcoes), "vinte e um");
        assert_eq!(inteiro_por_extenso(99, opcoes), "noventa e nove");
    }

    #[test]
    fn centenas() {
        let opcoes = OpcoesExtenso::default();
        assert_eq!(inteiro_por_extenso(100, opcoes), "cem");
        assert_eq!(inteiro_por_extenso(101, opcoes), "cento e um");
        assert_eq!(inteiro_por_extenso(200, opcoes), "duzentos");
        assert_eq!(inteiro_por_extenso(999, opcoes), "novecentos e noventa e nove");
    }

    #[test]
    fn milhares() {
        let opcoes = OpcoesExtenso::default();
        assert_eq!(inteiro_por_extenso(1000, opcoes), "mil");
        // Correção do bug do Vozz: era "mil, novecentos e noventa e oito"
        assert_eq!(inteiro_por_extenso(1998, opcoes), "mil novecentos e noventa e oito");
        assert_eq!(inteiro_por_extenso(2025, opcoes), "dois mil e vinte e cinco");
        // Múltiplo de 100 usa "e"
        assert_eq!(inteiro_por_extenso(1200, opcoes), "mil e duzentos");
        // Não múltiplo usa espaço
        assert_eq!(inteiro_por_extenso(1234, opcoes), "mil duzentos e trinta e quatro");
        assert_eq!(inteiro_por_extenso(1_000_000, opcoes), "um milhão");
        assert_eq!(inteiro_por_extenso(2_000_000, opcoes), "dois milhões");
        // Milhão + milhar + unidade
        assert_eq!(
            inteiro_por_extenso(1_234_567, opcoes),
            "um milhão duzentos e trinta e quatro mil quinhentos e sessenta e sete"
        );
    }

    #[test]
    fn negativo() {
        let opcoes = OpcoesExtenso::default();
        assert_eq!(inteiro_por_extenso(-5, opcoes), "menos cinco");
    }

    #[test]
    fn feminino() {
        let opcoes = OpcoesExtenso { feminino: true };
        assert_eq!(inteiro_por_extenso(1, opcoes), "uma");
        assert_eq!(inteiro_por_extenso(2, opcoes), "duas");
        assert_eq!(inteiro_por_extenso(200, opcoes), "duzentas");
        assert_eq!(inteiro_por_extenso(22, opcoes), "vinte e duas");
    }

    #[test]
    fn decimais() {
        let opcoes = OpcoesExtenso::default();
        assert_eq!(decimal_por_extenso(3, "14", opcoes), "três vírgula quatorze");
        assert_eq!(decimal_por_extenso(0, "05", opcoes), "zero vírgula zero cinco");
        assert_eq!(decimal_por_extenso(1, "5", opcoes), "um vírgula cinco");
        assert_eq!(decimal_por_extenso(10, "00", opcoes), "dez");
    }

    #[test]
    fn ordinais() {
        let opcoes = OpcoesExtenso::default();
        assert_eq!(ordinal_por_extenso(1, opcoes), "primeiro");
        assert_eq!(ordinal_por_extenso(2, opcoes), "segundo");
        assert_eq!(ordinal_por_extenso(10, opcoes), "décimo");
        assert_eq!(ordinal_por_extenso(21, opcoes), "vigésimo primeiro");
        assert_eq!(ordinal_por_extenso(42, opcoes), "quadragésimo segundo");
    }

    #[test]
    fn ordinais_feminino() {
        let opcoes = OpcoesExtenso { feminino: true };
        assert_eq!(ordinal_por_extenso(1, opcoes), "primeira");
        assert_eq!(ordinal_por_extenso(42, opcoes), "quadragésima segunda");
    }

    #[test]
    fn digitos() {
        // O Vozz mantém caracteres não numéricos, exceto hífen e espaços.
        assert_eq!(soletrar_digitos("123"), "um dois três");
        assert_eq!(soletrar_digitos("9-8"), "nove oito");
        // Parênteses são preservados pela função isolada.
        assert_eq!(soletrar_digitos("(11) 9"), "( um um ) nove");
        // Telefone sem parênteses, como o normalizador costuma passar.
        assert_eq!(
            soletrar_digitos("11 99999-8888"),
            "um um nove nove nove nove nove oito oito oito oito"
        );
    }

    #[test]
    fn anos() {
        assert_eq!(ano_por_extenso(2025), "dois mil e vinte e cinco");
        assert_eq!(ano_por_extenso(1900), "mil e novecentos");
    }
}