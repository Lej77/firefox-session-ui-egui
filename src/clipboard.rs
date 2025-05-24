#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(target_family = "wasm")]
#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    /// <https://developer.mozilla.org/en-US/docs/Web/API/Clipboard/writeText>
    #[wasm_bindgen(catch, js_name = "writeText", js_namespace = ["navigator", "clipboard"])]
    async fn write_text_to_web_clipboard(text: &str) -> Result<(), JsValue>;
}
#[cfg(target_family = "wasm")]
pub async fn write_text_to_clipboard(text: &str) -> Result<(), String> {
    write_text_to_web_clipboard(text)
        .await
        .map_err(|e| e.as_string().unwrap_or_default())
}

#[cfg(not(target_family = "wasm"))]
static CLIPBOARD: std::sync::Mutex<Option<arboard::Clipboard>> = std::sync::Mutex::new(None);
#[cfg(not(target_family = "wasm"))]
pub async fn write_text_to_clipboard(text: &str) -> Result<(), String> {
    let mut guard = CLIPBOARD.lock().unwrap();
    let clipboard = if let Some(clipboard) = &mut *guard {
        clipboard
    } else {
        let clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        guard.insert(clipboard)
    };
    clipboard.set_text(text).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn cleanup_clipboard() {
    #[cfg(not(target_family = "wasm"))]
    {
        if let Ok(mut guard) = CLIPBOARD.lock() {
            *guard = None; // drop the clipboard
        }
    }
}
