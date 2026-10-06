//! Vozz G2P em Rust — conversor grafema→fonema pt-BR.
//!
//! Pipeline: normalizar → dividir em sentenças → tagger → fonemizar → IPA.
//!
//! Módulos:
//!
//! - `numbers`:            extenso de números, decimais, ordinais, dígitos.
//! - `lexicon_palavra`:    léxico de exceções (modo isolado).
//! - `lexicon_contexto`:   léxico de exceções (modo contexto).
//! - `lexicon_homografos`: mapa `(palavra, sentido) → IPA`.
//! - `tagger`:             wrapper sobre o BCDE-tagger.
//! - `trema`:              regra do trema (Acordo Ortográfico de 1990).
//! - `piper`:              conversão IPA → tokens do Piper (pt-BR).
//! - `normalize`:          normalização de texto (datas, horas, moedas).
//! - `g2p`:                núcleo do conversor grafema→fonema.
//! - `splitter`:           divisão de texto em sentenças.

pub mod g2p;
pub mod lexicon_contexto;
pub mod lexicon_homografos;
pub mod lexicon_palavra;
pub mod normalize;
pub mod numbers;
pub mod piper;
pub mod splitter;
pub mod tagger;
pub mod trema;

pub use g2p::{
    acentuar, fonemizar, phonemize, palavra_para_ipa, silabificar, OpcoesFonemizar,
    Silaba,
};
pub use normalize::{normalizar, OpcoesNormalizar};
pub use piper::{ipa_para_piper, ipa_para_piper_str};
pub use splitter::dividir_em_sentencas;
