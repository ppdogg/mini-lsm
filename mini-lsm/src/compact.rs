use std::sync::Arc;

use anyhow::Result;

use crate::{
    iterators::{merge_iterator::MergeIterator, StorageIterator},
    lsm_storage::LsmStorage,
    table::{SsTable, SsTableBuilder, SsTableIterator},
};

struct CompactOptions {
    block_size: usize,
    target_sst_size: usize,
}

impl LsmStorage {
    fn compact(
        &self,
        tables: Vec<Arc<SsTable>>,
        options: CompactOptions,
    ) -> Result<Vec<Arc<SsTable>>> {
        let mut iters = Vec::new();
        iters.reserve(tables.len());
        for table in tables.iter() {
            iters.push(Box::new(SsTableIterator::create_and_seek_to_first(
                table.clone(),
            )?));
        }
        let mut iter = MergeIterator::create(iters);

        let mut builder = None;
        let mut new_sst = vec![];

        while iter.is_valid() {
            if builder.is_none() {
                builder = Some(SsTableBuilder::new(options.block_size));
            }
            let mut builder_inner = builder.as_mut().unwrap();
            builder_inner.add(iter.key(), iter.value());
            if builder_inner.estimated_size() >= options.target_sst_size {
                let sst_id = self.next_sst_id(); // lock dropped here
                let builder = builder.take().unwrap();
                let sst = Arc::new(builder.build(
                    sst_id,
                    Some(self.block_cache.clone()),
                    self.path_of_sst(sst_id),
                )?);
                new_sst.push(sst);
            }
        }
        if let Some(builder) = builder {
            let sst_id = self.next_sst_id(); // lock dropped here
            let sst = Arc::new(builder.build(
                sst_id,
                Some(self.block_cache.clone()),
                self.path_of_sst(sst_id),
            )?);
            new_sst.push(sst);
        }
        Ok(new_sst)
    }
}
