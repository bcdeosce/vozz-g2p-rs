pub mod g2p;
pub mod lexicon_contexto;
pub mod lexicon_homografos;
pub mod lexicon_palavra;
pub mod normalize;
pub mod numbers;
pub mod piper;
pub mod piper_pipeline;
pub mod splitter;
pub mod tagger;
pub mod trema;

pub use g2p::{
    acentuar, fonemizar, phonemize, palavra_para_ipa, silabificar,
    OpcoesFonemizar, Silaba,
};
pub use normalize::{normalizar, OpcoesNormalizar};
pub use piper::{ipa_para_piper, ipa_para_piper_str};
pub use piper_pipeline::{preparar_chunks, Chunk, Fragmento};
pub use splitter::dividir_em_sentencas;
