//! Wrapper sobre o binário BCDE-tagger (subprocesso persistente).
//!
//! O BCDE-tagger é compilado como binário separado. Este wrapper inicia
//! o processo e comunica via stdin/stdout, uma frase por linha.
//!
//! O binário é procurado em:
//!   1. variável de ambiente `BCDE_TAGGER_BIN` (caminho completo)
//!   2. `bcde-tagger` no PATH
//!
//! O diretório de dados é passado como primeiro argumento do binário.
//! Pode ser configurado via `BCDE_TAGGER_DATA` (padrão: `data`).

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::rc::Rc;

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

#[derive(Clone)]
pub struct Tagger {
    inner: Rc<RefCell<Subprocesso>>,
}

impl Tagger {
    /// Inicia o binário BCDE-tagger.
    ///
    /// `data_dir` é o diretório que contém os JSONs do tagger
    /// (`tabelas_v3.json`, `crf_weights.json`, etc.).
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

    /// Anota uma frase. Devolve os tokens com POS, diacrítico e sentido.
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
                break; // EOF
            }
            if linha.trim().is_empty() {
                break; // fim da frase
            }
            if let Some(tok) = parse_linha(&linha) {
                tokens.push(tok);
            }
        }
        Ok(tokens)
    }
}

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
