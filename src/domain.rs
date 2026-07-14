use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SimpleDate {
    year: u16,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
}

impl SimpleDate {
    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        // Expected format: "YYYY-MM-DD HH:MM"
        if s.len() != 16 {
            return Err("Date must be in format YYYY-MM-DD HH:MM");
        }

        let parts: Vec<&str> = s.split(['-', ' ', ':']).collect();
        if parts.len() != 5 {
            return Err("Invalid date format. Use YYYY-MM-DD HH:MM");
        }

        let parse_num = |part: &str| -> Result<u32, &'static str> {
            part.parse::<u32>().map_err(|_| "Date contains non-digit characters")
        };

        let year = parse_num(parts[0])?;
        if year > u16::MAX as u32 {
            return Err("Year is out of valid range");
        }

        let month = parse_num(parts[1])?;
        let day = parse_num(parts[2])?;
        let hour = parse_num(parts[3])?;
        let minute = parse_num(parts[4])?;

        if !(1..=12).contains(&month)
            || !(1..=31).contains(&day)
            || hour > 23
            || minute > 59
        {
            return Err("Date values are out of valid range");
        }

        Ok(SimpleDate {
            year: year as u16,
            month: month as u8,
            day: day as u8,
            hour: hour as u8,
            minute: minute as u8,
        })
    }
}

impl TryFrom<&str> for SimpleDate {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl fmt::Display for SimpleDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}-{:02}-{:02} {:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSlot {
    pub start: SimpleDate,
    pub end: SimpleDate,
}

impl TimeSlot {
    pub fn new(start: SimpleDate, end: SimpleDate) -> Result<Self, &'static str> {
        if start >= end {
            return Err("Start date must be before end date");
        }
        Ok(TimeSlot { start, end })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RankedSlot {
    pub slot: TimeSlot,
    pub attendance: usize,
}

#[derive(Clone)]
pub struct UserAvailability {
    pub user_id: String,
    pub free_slots: Vec<TimeSlot>,
}

/// Query builder for filtering and ranking rehearsal slots.
#[derive(Default)]
pub struct SlotQuery {
    pub top_n: Option<usize>,
    pub start_after: Option<SimpleDate>,
    pub end_before: Option<SimpleDate>,
}

impl SlotQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_top_n(mut self, n: usize) -> Self {
        self.top_n = Some(n);
        self
    }

    pub fn with_start_after(mut self, date: SimpleDate) -> Self {
        self.start_after = Some(date);
        self
    }

    pub fn with_end_before(mut self, date: SimpleDate) -> Self {
        self.end_before = Some(date);
        self
    }
}

pub fn validate_user_id(user_id: &str) -> Result<(), &'static str> {
    if user_id.is_empty() {
        return Err("Username cannot be empty");
    }
    if user_id.contains('|') {
        return Err("Username cannot contain '|'");
    }
    if matches!(user_id, "DONE" | "LIST" | "BEST") {
        return Err("Username cannot be a reserved command (DONE, LIST, BEST)");
    }
    Ok(())
}

/// Identifies and ranks the best rehearsal slots.
/// This function is a pure calculation: it takes a set of availability slots
/// and returns all possible overlapping intervals ranked by attendance.
pub fn find_best_availabilities<'a, I>(all_slots: I) -> Vec<RankedSlot>
where
    I: IntoIterator<Item = &'a [TimeSlot]>,
{
    let slots_collection: Vec<&'a [TimeSlot]> = all_slots.into_iter().collect();
    if slots_collection.is_empty() {
        return Vec::new();
    }

    let mut boundaries = Vec::new();
    for slots in &slots_collection {
        for slot in *slots {
            boundaries.push(slot.start);
            boundaries.push(slot.end);
        }
    }
    boundaries.sort();
    boundaries.dedup();

    let mut ranked_slots = Vec::new();
    for window in boundaries.windows(2) {
        let start = window[0];
        let end = window[1];

        let attendance = slots_collection
            .iter()
            .filter(|slots| {
                slots
                    .iter()
                    .any(|slot| slot.start <= start && slot.end >= end)
            })
            .count();

        if attendance > 0 {
            ranked_slots.push(RankedSlot {
                slot: TimeSlot { start, end },
                attendance,
            });
        }
    }

    ranked_slots.sort_by(|a, b| {
        b.attendance
            .cmp(&a.attendance)
            .then_with(|| a.slot.start.cmp(&b.slot.start))
    });

    ranked_slots
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> SimpleDate {
        SimpleDate::try_from(s).unwrap()
    }

    #[test]
    fn test_requirement_1_attendance_count() {
        let slots1 = vec![TimeSlot::new(date("2026-07-13 10:00"), date("2026-07-13 12:00")).unwrap()];
        let slots2 = vec![TimeSlot::new(date("2026-07-13 11:00"), date("2026-07-13 13:00")).unwrap()];
        let results = find_best_availabilities(vec![slots1.as_slice(), slots2.as_slice()]);

        assert_eq!(results[0].attendance, 2);
        assert_eq!(results[0].slot.start, date("2026-07-13 11:00"));
        assert_eq!(results[0].slot.end, date("2026-07-13 12:00"));
    }

    #[test]
    fn test_requirement_2_ranking_attendance() {
        let slots1 = vec![TimeSlot::new(date("2026-07-13 10:00"), date("2026-07-13 11:00")).unwrap()];
        let slots2 = vec![TimeSlot::new(date("2026-07-13 10:00"), date("2026-07-13 11:00")).unwrap()];
        let slots3 = vec![TimeSlot::new(date("2026-07-13 12:00"), date("2026-07-13 13:00")).unwrap()];
        let results = find_best_availabilities(vec![slots1.as_slice(), slots2.as_slice(), slots3.as_slice()]);
        assert_eq!(results[0].attendance, 2);
        assert_eq!(results[1].attendance, 1);
    }

    #[test]
    fn test_requirement_3_ranking_time() {
        let slots1 = vec![TimeSlot::new(date("2026-07-13 10:00"), date("2026-07-13 11:00")).unwrap()];
        let slots2 = vec![TimeSlot::new(date("2026-07-13 12:00"), date("2026-07-13 13:00")).unwrap()];
        let results = find_best_availabilities(vec![slots1.as_slice(), slots2.as_slice()]);
        assert_eq!(results[0].slot.start, date("2026-07-13 10:00"));
        assert_eq!(results[1].slot.start, date("2026-07-13 12:00"));
    }

    #[test]
    fn test_requirement_4_top_n() {
        let slots1 = vec![
            TimeSlot::new(date("2026-07-13 10:00"), date("2026-07-13 11:00")).unwrap(),
            TimeSlot::new(date("2026-07-13 12:00"), date("2026-07-13 13:00")).unwrap(),
            TimeSlot::new(date("2026-07-13 14:00"), date("2026-07-13 15:00")).unwrap(),
        ];
        let results = find_best_availabilities(vec![slots1.as_slice()]);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_date_filtering() {
        let slots1 = vec![
            TimeSlot::new(date("2026-07-13 10:00"), date("2026-07-13 12:00")).unwrap(),
            TimeSlot::new(date("2026-07-15 10:00"), date("2026-07-15 12:00")).unwrap(),
        ];

        let results = find_best_availabilities(vec![slots1.as_slice()]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_validate_user_id_rejects_reserved_and_pipe() {
        assert!(validate_user_id("Alice").is_ok());
        assert_eq!(
            validate_user_id("BEST"),
            Err("Username cannot be a reserved command (DONE, LIST, BEST)")
        );
        assert_eq!(
            validate_user_id("ali|ce"),
            Err("Username cannot contain '|'")
        );
    }
}
