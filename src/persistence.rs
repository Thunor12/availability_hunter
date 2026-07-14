use crate::domain::{SimpleDate, TimeSlot, UserAvailability};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;

/// Trait defining the persistence interface for user availability.
pub trait AvailabilityRepository {
    fn load_all(&self) -> io::Result<Vec<UserAvailability>>;
    fn save_all(&self, availabilities: &[UserAvailability]) -> io::Result<()>;
    fn save_slot(&self, user_id: &str, slot: TimeSlot) -> io::Result<()>;
}

/// A simple file-based implementation of the AvailabilityRepository.
/// Format: username|start_date|end_date
pub struct FileAvailabilityRepository {
    path: String,
}

impl FileAvailabilityRepository {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }
}

impl AvailabilityRepository for FileAvailabilityRepository {
    fn save_slot(&self, user_id: &str, slot: TimeSlot) -> io::Result<()> {
        let mut availabilities = self.load_all()?;
        if let Some(user) = availabilities.iter_mut().find(|u| u.user_id == user_id) {
            user.free_slots.push(slot);
        } else {
            availabilities.push(UserAvailability {
                user_id: user_id.to_string(),
                free_slots: vec![slot],
            });
        }
        self.save_all(&availabilities)
    }

    fn load_all(&self) -> io::Result<Vec<UserAvailability>> {
        let path = Path::new(&self.path);
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut users_map: std::collections::HashMap<String, Vec<TimeSlot>> =
            std::collections::HashMap::new();

        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() != 3 {
                continue;
            }

            let username = parts[0].to_string();
            let start = match SimpleDate::from_str(parts[1]) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let end = match SimpleDate::from_str(parts[2]) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if let Ok(slot) = TimeSlot::new(start, end) {
                users_map.entry(username).or_default().push(slot);
            }
        }

        Ok(users_map
            .into_iter()
            .map(|(id, slots)| UserAvailability {
                user_id: id,
                free_slots: slots,
            })
            .collect())
    }

    fn save_all(&self, availabilities: &[UserAvailability]) -> io::Result<()> {
        let tmp_path = format!("{}.tmp", self.path);
        {
            let mut file = File::create(&tmp_path)?;

            for user in availabilities {
                for slot in &user.free_slots {
                    writeln!(file, "{}|{}|{}", user.user_id, slot.start, slot.end)?;
                }
            }
            file.sync_all()?;
        }

        std::fs::rename(tmp_path, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SimpleDate;

    fn date(s: &str) -> SimpleDate {
        SimpleDate::try_from(s).unwrap()
    }

    fn cleanup(path: &str) {
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn test_requirement_5_save_load_cycle() {
        let test_path = "test_availability.db";
        let repo = FileAvailabilityRepository::new(test_path);

        let original_data = vec![
            UserAvailability {
                user_id: "Alice".to_string(),
                free_slots: vec![
                    TimeSlot::new(date("2026-07-13 10:00"), date("2026-07-13 12:00")).unwrap(),
                    TimeSlot::new(date("2026-07-13 18:00"), date("2026-07-13 20:00")).unwrap(),
                ],
            },
            UserAvailability {
                user_id: "Bob".to_string(),
                free_slots: vec![
                    TimeSlot::new(date("2026-07-13 11:00"), date("2026-07-13 13:00")).unwrap(),
                ],
            },
        ];

        repo.save_all(&original_data).expect("Save failed");
        let loaded_data = repo.load_all().expect("Load failed");

        assert_eq!(loaded_data.len(), 2);

        let alice = loaded_data
            .iter()
            .find(|u| u.user_id == "Alice")
            .expect("Alice not found");
        let bob = loaded_data
            .iter()
            .find(|u| u.user_id == "Bob")
            .expect("Bob not found");

        assert_eq!(alice.free_slots.len(), 2);
        assert_eq!(bob.free_slots.len(), 1);

        cleanup(test_path);
    }

    #[test]
    fn test_requirement_6_missing_file() {
        let repo = FileAvailabilityRepository::new("non_existent_file.db");
        let result = repo.load_all().expect("Load failed");
        assert!(result.is_empty());
    }

    #[test]
    fn test_requirement_7_malformed_data() {
        let test_path = "malformed_test.db";
        {
            let mut file = File::create(test_path).unwrap();
            writeln!(file, "Alice|2026-07-13 10:00|2026-07-13 12:00").unwrap();
            writeln!(file, "Bob|2026-07-13 10:00").unwrap();
            writeln!(file, "Charlie|not-a-date|2026-07-13 12:00").unwrap();
            writeln!(file, "Alice|2026-07-13 18:00|2026-07-13 20:00").unwrap();
        }

        let repo = FileAvailabilityRepository::new(test_path);
        let loaded = repo.load_all().expect("Load failed");

        assert_eq!(loaded.len(), 1);
        let alice = loaded.iter().find(|u| u.user_id == "Alice").unwrap();
        assert_eq!(alice.free_slots.len(), 2);

        cleanup(test_path);
    }

    #[test]
    fn test_save_slot_uses_atomic_write() {
        let test_path = "save_slot_atomic.db";
        let repo = FileAvailabilityRepository::new(test_path);

        let slot1 = TimeSlot::new(date("2026-07-13 10:00"), date("2026-07-13 12:00")).unwrap();
        let slot2 = TimeSlot::new(date("2026-07-14 10:00"), date("2026-07-14 12:00")).unwrap();

        repo.save_slot("Alice", slot1).expect("first save failed");
        repo.save_slot("Bob", slot2).expect("second save failed");

        let loaded = repo.load_all().expect("load failed");
        assert_eq!(loaded.len(), 2);

        let slot3 = TimeSlot::new(date("2026-07-15 10:00"), date("2026-07-15 12:00")).unwrap();
        repo.save_slot("Alice", slot3).expect("third save failed");

        let loaded = repo.load_all().expect("load failed");
        let alice = loaded.iter().find(|u| u.user_id == "Alice").unwrap();
        assert_eq!(alice.free_slots.len(), 2);

        cleanup(test_path);
    }
}
