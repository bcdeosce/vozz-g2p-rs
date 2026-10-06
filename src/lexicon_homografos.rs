//! Mapa `(palavra, sentido) → IPA`, sem desambiguação.
//! A desambiguação é feita pelo BCDE-tagger.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Deserialize)]
struct EntradaSentido {
    ipa: String,
    #[serde(default)] #[allow(dead_code)] confianca: f64,
    #[serde(default)] #[allow(dead_code)] ocorrencias: usize,
}

pub struct LexiconHomografos {
    lexico: HashMap<String, HashMap<String, String>>,
}

impl LexiconHomografos {
    pub fn from_path(path: &Path) -> Result<Self, String> {
        let texto = std::fs::read_to_string(path)
            .map_err(|e| format!("Erro ao ler {}: {}", path.display(), e))?;
        let bruto: HashMap<String, HashMap<String, EntradaSentido>> =
            serde_json::from_str(&texto)
                .map_err(|e| format!("Erro ao parsear léxico: {}", e))?;
        let lexico = bruto.into_iter().map(|(palavra, sentidos)| {
            let mapa: HashMap<String, String> = sentidos.into_iter()
                .map(|(s, d)| (s, d.ipa)).collect();
            (palavra, mapa)
        }).collect();
        Ok(Self { lexico })
    }
    pub fn ipa_para(&self, palavra: &str, sentido: &str) -> Option<&str> {
        self.lexico.get(palavra).and_then(|m| m.get(sentido))
            .map(|s| s.as_str())
    }
    pub fn tem_palavra(&self, palavra: &str) -> bool { self.lexico.contains_key(palavra) }
    pub fn len(&self) -> usize { self.lexico.len() }
    pub fn is_empty(&self) -> bool { self.lexico.is_empty() }
}
