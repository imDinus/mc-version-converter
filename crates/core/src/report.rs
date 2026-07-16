#[derive(Debug, Default)]
pub struct Report {
    pub warnings: Vec<String>,
    pub region_files: u64,
    pub chunks_rewritten: u64,
    pub blocks_replaced: u64,
    pub items_converted: u64,
    pub dropped_data: u64,
    pub chunks_skipped: u64,
    pub block_entities_added: u64,
}

impl Report {
    pub fn warn(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }

    pub fn merge(&mut self, other: Report) {
        self.warnings.extend(other.warnings);
        self.region_files += other.region_files;
        self.chunks_rewritten += other.chunks_rewritten;
        self.blocks_replaced += other.blocks_replaced;
        self.items_converted += other.items_converted;
        self.dropped_data += other.dropped_data;
        self.chunks_skipped += other.chunks_skipped;
        self.block_entities_added += other.block_entities_added;
    }

    pub fn dedup_warnings(&mut self) {
        let mut seen = std::collections::HashSet::new();
        self.warnings.retain(|w| seen.insert(w.clone()));
    }
}
