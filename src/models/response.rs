use crate::models::ResponseConfig;

use super::types::ResponseEntry;

impl ResponseConfig {
    pub fn get_entry_for_status(&self, status: u16) -> Option<&ResponseEntry> {
        let code_str = status.to_string();
        if let Some(entry) = self.codes.get(&code_str) {
            return Some(entry);
        }

        match status {
            200..=299 => self.success.as_ref(),
            300..=399 => self.warn.as_ref(),
            400..=599 => self.failure.as_ref(),
            _ => None,
        }
    }
}
