use crate::models::{NativeOverviewView, NativeSnapshotPage, NativeSnapshotTransfer};

#[Service]
#[derive(Default)]
pub struct NativeSnapshotTransportService {
    id: i64,
    content: String,
    offsets: Vec<usize>,
}

impl NativeSnapshotTransportService {
    const PAGE_BYTES: usize = 8192;
    const MAXIMUM_BYTES: usize = 2_097_152;
    const FIRST_OFFSET: usize = 0;

    pub fn open_view(&mut self, view: &NativeOverviewView) -> NativeSnapshotTransfer {
        self.clear();
        let encoded: Result<String, serde_json::Error> = serde_json::to_string(view);
        let validated: Result<String, String> = Self::validated_content(encoded);
        self.accept(validated)
    }

    pub fn page(&self, id: i64, index: i64) -> NativeSnapshotPage {
        self.find_page(id, index)
    }

    pub fn release(&mut self, id: i64) -> bool {
        self.release_current(id)
    }

    pub fn retire(&mut self) -> () {
        self.id = self.id.saturating_add(1);
        self.clear();
    }

    fn validated_content(encoded: Result<String, serde_json::Error>) -> Result<String, String> {
        encoded
            .map_err(Self::encoding_error)
            .and_then(Self::size_checked)
    }

    fn encoding_error(_error: serde_json::Error) -> String {
        "Cannot encode the local snapshot.".to_owned()
    }

    fn size_checked(content: String) -> Result<String, String> {
        if content.len() > Self::MAXIMUM_BYTES {
            return Err("Local snapshot exceeds its 2 MiB transfer limit.".to_owned());
        }
        Ok(content)
    }

    fn accept(&mut self, encoded: Result<String, String>) -> NativeSnapshotTransfer {
        match encoded {
            Ok(content) => self.save(content),
            Err(error) => Self::failure(&error),
        }
    }

    fn save(&mut self, content: String) -> NativeSnapshotTransfer {
        self.id = self.id.saturating_add(1);
        self.content = content;
        self.offsets = Self::page_offsets(&self.content);
        NativeSnapshotTransfer {
            id: self.id,
            pages: (self.offsets.len() - 1) as i64,
            bytes: self.content.len() as i64,
            error: String::new(),
        }
    }

    fn page_offsets(content: &str) -> Vec<usize> {
        let mut offsets: Vec<usize> = vec![Self::FIRST_OFFSET];
        while *offsets.last().unwrap_or(&Self::FIRST_OFFSET) < content.len() {
            let previous: usize = *offsets.last().unwrap_or(&Self::FIRST_OFFSET);
            offsets.push(Self::page_end(content, previous));
        }
        offsets
    }

    fn page_end(content: &str, start: usize) -> usize {
        let mut end: usize = (start + Self::PAGE_BYTES).min(content.len());
        while !content.is_char_boundary(end) {
            end -= 1;
        }
        end
    }

    fn find_page(&self, id: i64, index: i64) -> NativeSnapshotPage {
        let bounds: Option<(usize, usize)> = self.page_bounds(id, index);
        self.selected_page(id, index, bounds)
    }

    fn page_bounds(&self, id: i64, index: i64) -> Option<(usize, usize)> {
        self.matching_transfer(id)?;
        let index: usize = usize::try_from(index).ok()?;
        let next: usize = index.checked_add(1)?;
        let start: usize = *self.offsets.get(index)?;
        let end: usize = *self.offsets.get(next)?;
        Some((start, end))
    }

    fn matching_transfer(&self, id: i64) -> Option<()> {
        if id != self.id {
            return None;
        }
        Some(())
    }

    fn selected_page(
        &self,
        id: i64,
        index: i64,
        bounds: Option<(usize, usize)>,
    ) -> NativeSnapshotPage {
        match bounds {
            Some(bounds) => self.page_content(id, index, bounds),
            None => Self::missing_page(id, index),
        }
    }

    fn page_content(&self, id: i64, index: i64, bounds: (usize, usize)) -> NativeSnapshotPage {
        NativeSnapshotPage {
            id,
            index,
            content: self.content[bounds.0..bounds.1].to_owned(),
            error: String::new(),
        }
    }

    fn missing_page(id: i64, index: i64) -> NativeSnapshotPage {
        NativeSnapshotPage {
            id,
            index,
            error: String::from("Snapshot transfer expired or page is unavailable."),
            ..Default::default()
        }
    }

    fn release_current(&mut self, id: i64) -> bool {
        if id != self.id {
            return false;
        }
        self.clear();
        true
    }

    fn clear(&mut self) -> () {
        self.content.clear();
        self.offsets.clear();
    }

    fn failure(message: &str) -> NativeSnapshotTransfer {
        NativeSnapshotTransfer {
            error: message.to_owned(),
            ..Default::default()
        }
    }
}
