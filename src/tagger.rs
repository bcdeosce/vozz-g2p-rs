//! Wrapper sobre o binário BCDE-tagger (subprocesso persistente).
//!
//! O BCDE-tagger é compilado como binário separado (não expõe `[lib]`).
//! Este wrapper inicia o processo uma vez e comunica via stdin/stdout,
//! uma frase por linha. O tagger devolve os tokens anotados e uma
//! linha vazia marca o fim da frase.
//!
//! Variáveis de ambiente:
//!
//! - `BCDE_TAGGER_BIN`  — caminho do binário (default: `bcde-tagger` no PATH).
//! - `BCDE_TAGGER_DATA` — diretório de dados passado como 1º argumento.

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::rc::Rc;

/// Token anotado pelo tagger.
#[derive(Debug, Clone)]
pub struct Token {
    pub word: String,
    pub upos: String,
    pub diacritic: Option<String>,
    pub sense: Option<String>,
    pub resolver_level: Option<String>,
}

struct Subprocesso {
    #[allow(dead_code)]
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

/// Handle clonável sobre o subprocesso. `tag` recebe `&self` para
/// simplificar o uso em `OpcoesFonemizar`.
#[derive(Clone)]
pub struct Tagger {
    inner: Rc<RefCell<Subprocesso>>,
}

impl Tagger {
    /// Inicia o binário BCDE-tagger apontando para `data_dir`.
    pub fn load(data_dir: &str) -> Result<Self, String> {
        let bin = std::env::var("BCDE_TAGGER_BIN")
            .unwrap_or_else(|_| "bcde-tagger".to_string());

        let mut child = Command::new(&bin)
            .arg(data_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("falha ao iniciar {}: {}", bin, e))?;

        let stdin = child.stdin.take().ok_or("stdin indisponível")?;
        let stdout = BufReader::new(
            child.stdout.take().ok_or("stdout indisponível")?,
        );

        Ok(Self {
            inner: Rc::new(RefCell::new(Subprocesso { child, stdin, stdout })),
        })
    }

    /// Anota uma frase inteira.
    ///
    /// Envia a frase e lê tokens até encontrar uma linha vazia (fim
    /// de resposta) ou EOF.
    pub fn tag(&self, texto: &str) -> Result<Vec<Token>, String> {
        let mut sub = self.inner.borrow_mut();

        writeln!(sub.stdin, "{}", texto).map_err(|e| e.to_string())?;
        sub.stdin.flush().map_err(|e| e.to_string())?;

        let mut tokens = Vec::new();
        loop {
            let mut linha = String::new();
            let n = sub
                .stdout
                .read_line(&mut linha)
                .map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            if linha.trim().is_empty() {
                break;
            }
            if let Some(tok) = parse_linha(&linha) {
                tokens.push(tok);
            }
        }
        Ok(tokens)
    }
}

/// Parser tolerante: aceita `word\tupos` seguido de campos opcionais
/// no formato `chave=valor` (ex.: `diac=...\tsense=...\tvia=...`).
fn parse_linha(linha: &str) -> Option<Token> {
    let partes: Vec<&str> = linha.trim().split('\t').collect();
    if partes.len() < 2 {
        return None;
    }
    let word = partes[0].to_string();
    let upos = partes[1].to_string();
    let mut diacritic = None;
    let mut sense = None;
    let mut resolver_level = None;

    for parte in &partes[2..] {
        if let Some(v) = parte.strip_prefix("diac=") {
            diacritic = Some(v.to_string());
        } else if let Some(v) = parte.strip_prefix("sense=") {
            sense = Some(v.to_string());
        } else if let Some(v) = parte.strip_prefix("via=") {
            resolver_level = Some(v.to_string());
        }
    }

    Some(Token {
        word,
        upos,
        diacritic,
        sense,
        resolver_level,
    })
}
