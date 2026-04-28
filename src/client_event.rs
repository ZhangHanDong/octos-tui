use octos_core::app_ui::AppUiEvent;

use crate::model::DiffPreviewGetResult;

#[derive(Debug, Clone)]
pub enum ClientEvent {
    App(Box<AppUiEvent>),
    DiffPreview(DiffPreviewGetResult),
}

impl From<AppUiEvent> for ClientEvent {
    fn from(event: AppUiEvent) -> Self {
        Self::App(Box::new(event))
    }
}
