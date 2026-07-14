mod domain;
mod persistence;
mod service;

use domain::SimpleDate;
use persistence::FileAvailabilityRepository;
use service::{RehearsalService, ServiceError};
use std::io::{self, Write};

enum AppCommand {
    AddSlot {
        user: String,
        start: SimpleDate,
        end: SimpleDate,
    },
    ListUsers,
    GetBestSlots {
        n: usize,
        filter: Option<SlotFilter>,
    },
    Exit,
}

enum SlotFilter {
    After(SimpleDate),
    Before(SimpleDate),
    Between(SimpleDate, SimpleDate),
}

fn parse_command(input: &str) -> Result<AppCommand, String> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.is_empty() {
        return Err("No input provided".to_string());
    }

    match parts[0] {
        "DONE" => Ok(AppCommand::Exit),
        "LIST" => Ok(AppCommand::ListUsers),
        "BEST" => {
            if parts.len() < 2 {
                return Err("Usage: BEST <n> [FILTER]".to_string());
            }
            let n = parts[1]
                .parse::<usize>()
                .map_err(|_| "Invalid number for N".to_string())?;
            if n == 0 {
                return Err("N must be greater than 0".to_string());
            }

            let filter = if parts.len() > 2 {
                match parts[2] {
                    "AFTER" if parts.len() == 5 => {
                        let date_str = format!("{} {}", parts[3], parts[4]);
                        Some(SlotFilter::After(
                            SimpleDate::from_str(&date_str).map_err(|e| e.to_string())?,
                        ))
                    }
                    "BEFORE" if parts.len() == 5 => {
                        let date_str = format!("{} {}", parts[3], parts[4]);
                        Some(SlotFilter::Before(
                            SimpleDate::from_str(&date_str).map_err(|e| e.to_string())?,
                        ))
                    }
                    "BETWEEN" if parts.len() == 8 && parts[5] == "AND" => {
                        let d1_str = format!("{} {}", parts[3], parts[4]);
                        let d2_str = format!("{} {}", parts[6], parts[7]);
                        let d1 = SimpleDate::from_str(&d1_str).map_err(|e| e.to_string())?;
                        let d2 = SimpleDate::from_str(&d2_str).map_err(|e| e.to_string())?;
                        if d1 > d2 {
                            return Err(
                                "BETWEEN start date must be before or equal to end date"
                                    .to_string(),
                            );
                        }
                        Some(SlotFilter::Between(d1, d2))
                    }
                    _ => return Err(
                        "Invalid filter. Use AFTER <date>, BEFORE <date>, or BETWEEN <d1> AND <d2>"
                            .to_string(),
                    ),
                }
            } else {
                None
            };
            Ok(AppCommand::GetBestSlots { n, filter })
        }
        _ => {
            if parts.len() == 5 {
                let user = parts[0].to_string();
                let start_str = format!("{} {}", parts[1], parts[2]);
                let end_str = format!("{} {}", parts[3], parts[4]);

                let start = SimpleDate::from_str(&start_str).map_err(|e| e.to_string())?;
                let end = SimpleDate::from_str(&end_str).map_err(|e| e.to_string())?;

                Ok(AppCommand::AddSlot { user, start, end })
            } else {
                Err(
                    "Invalid input. Use: <username> <start> <end>, LIST, BEST <n> [FILTER], or DONE"
                        .to_string(),
                )
            }
        }
    }
}

fn report_service_error(error: ServiceError) {
    eprintln!("Error: {error}");
}

fn main() {
    let repo = FileAvailabilityRepository::new("availability.db");
    let mut service = RehearsalService::new(repo);

    println!("Rehearsal Planner");
    println!("Enter availability as: <username> <YYYY-MM-DD HH:MM> <YYYY-MM-DD HH:MM>");
    println!("Enter 'LIST' to see all users currently in the system.");
    println!("Enter 'BEST <n>' to see the top N rehearsal slots.");
    println!("Enter 'BEST <n> AFTER <date>' or 'BEFORE <date>' or 'BETWEEN <d1> AND <d2>' to filter.");
    println!("Enter 'DONE' to exit. Availability is saved as you add it.");

    loop {
        print!("> ");
        if let Err(err) = io::stdout().flush() {
            eprintln!("Error: failed to write prompt: {err}");
            break;
        }

        let mut input = String::new();
        if let Err(err) = io::stdin().read_line(&mut input) {
            eprintln!("Error: failed to read input: {err}");
            break;
        }
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        match parse_command(input) {
            Ok(AppCommand::Exit) => break,
            Ok(AppCommand::ListUsers) => match service.get_all_users() {
                Ok(users) if users.is_empty() => {
                    println!("No users currently in the system.");
                }
                Ok(users) => {
                    println!("Current users: {}", users.join(", "));
                }
                Err(err) => report_service_error(err),
            },
            Ok(AppCommand::GetBestSlots { n, filter }) => {
                let mut query = domain::SlotQuery::new().with_top_n(n);
                if let Some(f) = filter {
                    query = match f {
                        SlotFilter::After(d) => query.with_start_after(d),
                        SlotFilter::Before(d) => query.with_end_before(d),
                        SlotFilter::Between(d1, d2) => {
                            query.with_start_after(d1).with_end_before(d2)
                        }
                    };
                }

                match service.get_best_slots(query) {
                    Ok(ranked) if ranked.is_empty() => {
                        println!("\nNo overlapping rehearsal slots found for these criteria.");
                    }
                    Ok(ranked) => {
                        println!("\nTop {} best rehearsal slots:", ranked.len());
                        for (i, item) in ranked.iter().enumerate() {
                            println!(
                                "{}. {} to {} (Attendance: {} users)",
                                i + 1,
                                item.slot.start,
                                item.slot.end,
                                item.attendance
                            );
                        }
                        println!();
                    }
                    Err(err) => report_service_error(err),
                }
            }
            Ok(AppCommand::AddSlot { user, start, end }) => {
                if let Err(err) = service.add_slot(user, start, end) {
                    report_service_error(err);
                }
            }
            Err(e) => println!("Error: {}", e),
        }
    }

    println!("Goodbye!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_rejects_best_zero() {
        assert!(parse_command("BEST 0").is_err());
    }

    #[test]
    fn parse_command_rejects_reversed_between_dates() {
        assert!(parse_command("BEST 3 BETWEEN 2026-07-15 12:00 AND 2026-07-13 10:00").is_err());
    }
}
