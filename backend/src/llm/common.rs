#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    TranscriptionModel,
    TranslationModel,
    /// Used for chunk summaries and speaker attribution.
    SummaryModel,
    /// Used for overlap detection and stitching.
    StitchModel,
}

impl Model {
    pub fn as_str(&self) -> &'static str {
        match self {
            Model::TranscriptionModel => "gemini-3-flash-preview",
            Model::TranslationModel => "gemini-3.1-flash-lite",
            Model::SummaryModel => "gemini-3.1-flash-lite",
            Model::StitchModel => "gemini-3.1-flash-lite",
        }
    }
}
