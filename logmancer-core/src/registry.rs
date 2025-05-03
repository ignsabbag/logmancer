use std::io;
use std::sync::Arc;
use dashmap::DashMap;
use dashmap::mapref::one::RefMut;
use uuid::Uuid;
use crate::LogReader;

pub struct LogRegistry {
    open_files: Arc<DashMap<Uuid, LogReader>>
}

impl LogRegistry {
    
    pub fn new() -> Self {
        LogRegistry {
            open_files: Arc::new(DashMap::new())
        }
    }
    
    /// Opens a new file and register with a UUID
    pub fn open_file(&self, path: &str) -> io::Result<String> {
        let uuid = Uuid::new_v4();
        let reader = LogReader::new(path.to_string());
        self.open_files.insert(uuid, reader?);
        Ok(uuid.to_string())
    }

    /// Gets a LogReader by UUID
    pub fn get_reader(&self, file_id: &str) -> Option<RefMut<Uuid,LogReader>> {
        if let Ok(uuid) = Uuid::parse_str(file_id) {
            self.open_files.get_mut(&uuid)
        } else {
            None
        }
    }
}