#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    TranscriptionModel,
    TranslationModel,
}

impl Model {
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::TranscriptionModel => "gemini-3-flash-preview",
            Model::TranslationModel => "gemini-3.1-flash-lite-preview",
        }
    }
}
