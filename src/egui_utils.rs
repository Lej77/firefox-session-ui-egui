use egui::TextBuffer;
use std::any::TypeId;

/// Used as a buffer for [`egui::TextEdit`] when the actual data is immutable
/// but we want the same styling as the mutable case.
pub struct FakeMutable<'a>(pub &'a str);
impl TextBuffer for FakeMutable<'_> {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.0
    }

    fn insert_text(&mut self, _text: &str, _ch_idx: usize) -> usize {
        0
    }

    fn delete_char_range(&mut self, _ch_range: std::ops::Range<usize>) {}

    fn type_id(&self) -> TypeId {
        unimplemented!()
    }
}

pub struct ObservableMutable<'a, T, F> {
    inner: &'a mut T,
    on_change: F,
}
impl<'a, T, F> ObservableMutable<'a, T, F> {
    pub fn new(inner: &'a mut T, on_change: F) -> Self
    where
        F: FnMut(&mut T),
    {
        Self { inner, on_change }
    }
    fn notify_change(&mut self)
    where
        F: FnMut(&mut T),
    {
        (self.on_change)(self.inner);
    }
}
impl<T, F> TextBuffer for ObservableMutable<'_, T, F>
where
    T: TextBuffer,
    F: FnMut(&mut T),
{
    fn is_mutable(&self) -> bool {
        TextBuffer::is_mutable(&*self.inner)
    }

    fn as_str(&self) -> &str {
        TextBuffer::as_str(&*self.inner)
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        self.notify_change();
        TextBuffer::insert_text(&mut *self.inner, text, char_index)
    }

    fn delete_char_range(&mut self, char_range: std::ops::Range<usize>) {
        self.notify_change();
        TextBuffer::delete_char_range(&mut *self.inner, char_range)
    }

    fn char_range(&self, char_range: std::ops::Range<usize>) -> &str {
        TextBuffer::char_range(&*self.inner, char_range)
    }

    fn byte_index_from_char_index(&self, char_index: usize) -> usize {
        TextBuffer::byte_index_from_char_index(&*self.inner, char_index)
    }

    fn clear(&mut self) {
        self.notify_change();
        TextBuffer::clear(&mut *self.inner)
    }

    fn replace_with(&mut self, text: &str) {
        self.notify_change();
        TextBuffer::replace_with(&mut *self.inner, text)
    }

    fn take(&mut self) -> String {
        self.notify_change();
        TextBuffer::take(&mut *self.inner)
    }

    fn insert_text_at(
        &mut self,
        ccursor: &mut egui::text::CCursor,
        text_to_insert: &str,
        char_limit: usize,
    ) {
        self.notify_change();
        TextBuffer::insert_text_at(&mut *self.inner, ccursor, text_to_insert, char_limit)
    }

    fn decrease_indentation(&mut self, ccursor: &mut egui::text::CCursor) {
        self.notify_change();
        TextBuffer::decrease_indentation(&mut *self.inner, ccursor)
    }

    fn delete_selected(
        &mut self,
        cursor_range: &egui::text_selection::CCursorRange,
    ) -> egui::text::CCursor {
        self.notify_change();
        TextBuffer::delete_selected(&mut *self.inner, cursor_range)
    }

    fn delete_selected_ccursor_range(
        &mut self,
        [min, max]: [egui::text::CCursor; 2],
    ) -> egui::text::CCursor {
        self.notify_change();
        TextBuffer::delete_selected_ccursor_range(&mut *self.inner, [min, max])
    }
    fn delete_previous_char(&mut self, ccursor: egui::text::CCursor) -> egui::text::CCursor {
        self.notify_change();
        TextBuffer::delete_previous_char(&mut *self.inner, ccursor)
    }

    fn delete_next_char(&mut self, ccursor: egui::text::CCursor) -> egui::text::CCursor {
        self.notify_change();
        TextBuffer::delete_next_char(&mut *self.inner, ccursor)
    }

    fn delete_previous_word(&mut self, max_ccursor: egui::text::CCursor) -> egui::text::CCursor {
        self.notify_change();
        TextBuffer::delete_previous_word(&mut *self.inner, max_ccursor)
    }

    fn delete_next_word(&mut self, min_ccursor: egui::text::CCursor) -> egui::text::CCursor {
        self.notify_change();
        TextBuffer::delete_next_word(&mut *self.inner, min_ccursor)
    }

    fn delete_paragraph_before_cursor(
        &mut self,
        galley: &egui::Galley,
        cursor_range: &egui::text_selection::CCursorRange,
    ) -> egui::text::CCursor {
        self.notify_change();
        TextBuffer::delete_paragraph_before_cursor(&mut *self.inner, galley, cursor_range)
    }

    fn delete_paragraph_after_cursor(
        &mut self,
        galley: &egui::Galley,
        cursor_range: &egui::text_selection::CCursorRange,
    ) -> egui::text::CCursor {
        self.notify_change();
        TextBuffer::delete_paragraph_after_cursor(&mut *self.inner, galley, cursor_range)
    }

    fn type_id(&self) -> TypeId {
        unimplemented!()
    }
}
