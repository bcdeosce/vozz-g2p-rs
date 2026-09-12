//! Vozz G2P em Rust — conversor grafema→fonema pt-BR.
//!
//! Pipeline: normalizar → dividir em sentenças → fonemizar → IPA.
//!
//! Módulos:
//!
//! - `numbers`:   extenso de números, decimais, ordinais, dígitos.
//! - `lexicon`:   léxico de exceções e clíticos.
//! - `normalize`: normalização de texto (datas, horas, moedas, siglas).
//! - `g2p`:       núcleo do conversor grafema→fonema.
//! - `splitter`:  divisão de texto em sentenças.

pub mod numbers;
pub mod lexicon;
pub mod normalize;
pub mod g2p;
pub mod splitter;

pub use g2p::{
    acentuar, fonemizar, phonemize, palavra_para_ipa, silabificar, OpcoesFonemizar,
    Silaba,
};
pub use lexicon::{buscar_clitico, buscar_lexico, CLITICOS, LEXICO};
pub use normalize::{normalizar, OpcoesNormalizar};
pub use splitter::dividir_em_sentencas;