use crate::domain::{
    find_best_availabilities, validate_user_id, RankedSlot, SimpleDate, SlotQuery, TimeSlot,
};
use crate::persistence::AvailabilityRepository;
use std::io;

#[derive(Debug)]
pub enum ServiceError {
    Validation(&'static str),
    Io(io::Error),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceError::Validation(msg) => write!(f, "{msg}"),
            ServiceError::Io(err) => write!(f, "Storage error: {err}"),
        }
    }
}

/// The Service layer coordinates domain logic and persistence.
pub struct RehearsalService<R: AvailabilityRepository> {
    repo: R,
}

impl<R: AvailabilityRepository> RehearsalService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub fn add_slot(
        &mut self,
        user_id: String,
        start: SimpleDate,
        end: SimpleDate,
    ) -> Result<(), ServiceError> {
        validate_user_id(&user_id).map_err(ServiceError::Validation)?;
        let slot = TimeSlot::new(start, end).map_err(ServiceError::Validation)?;
        self.repo
            .save_slot(&user_id, slot)
            .map_err(ServiceError::Io)?;
        Ok(())
    }

    pub fn get_best_slots(&self, query: SlotQuery) -> Result<Vec<RankedSlot>, ServiceError> {
        let users = self.repo.load_all().map_err(ServiceError::Io)?;
        let all_ranked = find_best_availabilities(users.iter().map(|u| u.free_slots.as_slice()));
        Ok(apply_slot_query(all_ranked, &query))
    }

    pub fn get_all_users(&self) -> Result<Vec<String>, ServiceError> {
        let mut users: Vec<String> = self
            .repo
            .load_all()
            .map_err(ServiceError::Io)?
            .into_iter()
            .map(|u| u.user_id)
            .collect();
        users.sort();
        Ok(users)
    }
}

fn apply_slot_query(ranked: Vec<RankedSlot>, query: &SlotQuery) -> Vec<RankedSlot> {
    let filtered: Vec<RankedSlot> = ranked
        .into_iter()
        .filter(|r| {
            let after_ok = query
                .start_after
                .is_none_or(|d| r.slot.start >= d);
            let before_ok = query.end_before.is_none_or(|d| r.slot.end <= d);
            after_ok && before_ok
        })
        .collect();

    let limit = query.top_n.unwrap_or(usize::MAX);
    filtered.into_iter().take(limit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{SimpleDate, UserAvailability};
    use std::cell::RefCell;

    fn date(s: &str) -> SimpleDate {
        SimpleDate::try_from(s).unwrap()
    }

    struct MockRepository {
        data: RefCell<Vec<UserAvailability>>,
    }

    impl MockRepository {
        fn with_users(users: Vec<UserAvailability>) -> Self {
            Self {
                data: RefCell::new(users),
            }
        }
    }

    impl AvailabilityRepository for MockRepository {
        fn load_all(&self) -> io::Result<Vec<UserAvailability>> {
            Ok(self.data.borrow().clone())
        }

        fn save_all(&self, availabilities: &[UserAvailability]) -> io::Result<()> {
            *self.data.borrow_mut() = availabilities.to_vec();
            Ok(())
        }

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
    }

    fn sample_users() -> Vec<UserAvailability> {
        vec![
            UserAvailability {
                user_id: "Alice".to_string(),
                free_slots: vec![
                    TimeSlot::new(date("2026-07-13 10:00"), date("2026-07-13 12:00")).unwrap(),
                    TimeSlot::new(date("2026-07-15 10:00"), date("2026-07-15 12:00")).unwrap(),
                ],
            },
            UserAvailability {
                user_id: "Bob".to_string(),
                free_slots: vec![
                    TimeSlot::new(date("2026-07-13 11:00"), date("2026-07-13 13:00")).unwrap(),
                ],
            },
        ]
    }

    #[test]
    fn test_requirement_4_top_n_limits_results() {
        let service = RehearsalService::new(MockRepository::with_users(sample_users()));
        let query = SlotQuery::new().with_top_n(1);
        let results = service.get_best_slots(query).expect("query failed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attendance, 2);
    }

    #[test]
    fn test_filter_after_date() {
        let service = RehearsalService::new(MockRepository::with_users(sample_users()));
        let query = SlotQuery::new()
            .with_top_n(10)
            .with_start_after(date("2026-07-15 00:00"));
        let results = service.get_best_slots(query).expect("query failed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slot.start, date("2026-07-15 10:00"));
    }

    #[test]
    fn test_filter_before_date() {
        let service = RehearsalService::new(MockRepository::with_users(sample_users()));
        let query = SlotQuery::new()
            .with_top_n(10)
            .with_end_before(date("2026-07-13 12:00"));
        let results = service.get_best_slots(query).expect("query failed");

        assert!(!results.is_empty());
        assert!(results.iter().all(|r| r.slot.end <= date("2026-07-13 12:00")));
    }

    #[test]
    fn test_filter_between_dates() {
        let service = RehearsalService::new(MockRepository::with_users(sample_users()));
        let query = SlotQuery::new()
            .with_top_n(10)
            .with_start_after(date("2026-07-13 11:00"))
            .with_end_before(date("2026-07-13 12:00"));
        let results = service.get_best_slots(query).expect("query failed");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].slot.start, date("2026-07-13 11:00"));
        assert_eq!(results[0].slot.end, date("2026-07-13 12:00"));
    }

    #[test]
    fn test_add_slot_rejects_invalid_username() {
        let mut service = RehearsalService::new(MockRepository::with_users(vec![]));
        let start = date("2026-07-13 10:00");
        let end = date("2026-07-13 12:00");

        let err = service
            .add_slot("BEST".to_string(), start, end)
            .expect_err("expected validation error");
        assert!(matches!(err, ServiceError::Validation(_)));
    }
}
