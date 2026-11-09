use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

use super::{Span, TextLayout, TextStyle};

/// Frames an entry can go unused before the cache drops it
const KEEP_FRAMES: u64 = 3;

/// One cached line and the spans it came from
struct Entry {
    text: Box<str>,
    /// Each span's length in bytes and style
    styles: Box<[(usize, TextStyle)]>,
    layout: Arc<TextLayout>,
    used: u64,
}

impl Entry {
    fn matches(&self, spans: &[Span]) -> bool {
        if self.styles.len() != spans.len() {
            return false;
        }

        let mut rest = &*self.text;
        for ((len, style), span) in self.styles.iter().zip(spans) {
            if *len != span.text.len() || *style != span.style || !rest.starts_with(span.text) {
                return false;
            }

            rest = &rest[*len..];
        }

        true
    }
}

/// Lines kept across frames
#[derive(Default)]
pub(super) struct LineCache {
    /// Entries grouped by the hash of their spans
    entries: HashMap<u64, Vec<Entry>>,
    frame: u64,
}

impl LineCache {
    pub fn get(&mut self, spans: &[Span]) -> Option<Arc<TextLayout>> {
        let frame = self.frame;
        let entry = self
            .entries
            .get_mut(&key(spans))?
            .iter_mut()
            .find(|e| e.matches(spans))?;

        entry.used = frame;
        Some(entry.layout.clone())
    }

    pub fn insert(&mut self, spans: &[Span], layout: Arc<TextLayout>) {
        let entry = Entry {
            text: spans.iter().map(|s| s.text).collect::<String>().into(),
            styles: spans
                .iter()
                .map(|s| (s.text.len(), s.style.clone()))
                .collect(),
            layout,
            used: self.frame,
        };

        self.entries.entry(key(spans)).or_default().push(entry);
    }

    /// Drops entries that the last few frames did not use
    pub fn end_frame(&mut self) {
        let frame = self.frame;
        self.entries.retain(|_, bucket| {
            bucket.retain(|e| frame - e.used < KEEP_FRAMES);
            !bucket.is_empty()
        });

        self.frame += 1;
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.values().map(Vec::len).sum()
    }
}

fn key(spans: &[Span]) -> u64 {
    let mut hasher = DefaultHasher::new();
    spans.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod test {
    use parley::Layout;

    use super::*;

    fn layout() -> Arc<TextLayout> {
        Arc::new(TextLayout::new(Layout::new()))
    }

    #[test]
    fn matches_on_text_and_styles() {
        let mut cache = LineCache::default();
        let bold = TextStyle::default().bold();
        let spans = [
            Span::new("fn", bold.clone()),
            Span::new(" main", TextStyle::default()),
        ];

        cache.insert(&spans, layout());

        assert!(cache.get(&spans).is_some());
        assert!(cache.get(&[Span::new("fn main", bold.clone())]).is_none());
        assert!(
            cache
                .get(&[
                    Span::new("f", bold),
                    Span::new("n main", TextStyle::default())
                ])
                .is_none()
        );
    }

    #[test]
    fn drops_lines_unused_for_a_few_frames() {
        let mut cache = LineCache::default();
        let kept = [Span::new("kept", TextStyle::default())];
        let dropped = [Span::new("dropped", TextStyle::default())];

        cache.insert(&kept, layout());
        cache.insert(&dropped, layout());

        for _ in 0..KEEP_FRAMES {
            cache.end_frame();
            cache.get(&kept);
        }
        cache.end_frame();

        assert_eq!(cache.len(), 1);
        assert!(cache.get(&kept).is_some());
    }
}
