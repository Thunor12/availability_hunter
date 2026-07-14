# Rehearsal Planner Requirements

## Project Goal
Create an application that allows multiple users to input their availability and automatically identifies the best time slots for rehearsals based on attendance and timing.

## Functional Requirements
- **User Availability Input**:
  - Users must be able to specify date/time ranges when they are free.
  - Support for multiple users.
  - Support for multiple time slots per user.
- **Overlap Detection & Ranking**:
  - The system must identify time windows where one or more users are free.
  - The system must rank these windows based on:
    1. **Highest Attendance**: Slots where the most users are free.
    2. **Earliest Time**: Among slots with equal attendance, the earliest one is preferred.
  - The system must be able to return the top N best slots.
- **Persistence**:
  - Availability must be persisted to a database (file-based) to maintain data across sessions.
  - The system must load existing data when queried and save new slots atomically as they are added.
- **Output/Visualization**:
  - Display a ranked list of the best rehearsal windows, including the number of users who can attend.

## Technical Requirements
- **Language**: Rust.
- **Dependencies**: Minimal to zero external dependencies.
- **Architecture**:
  - **Layered Architecture**: Clear separation between Interface (CLI/Web), Application Service, Domain, and Persistence.
  - **Service Layer**: A dedicated service layer to coordinate domain logic and persistence, exposing a clean API for various interfaces.
  - **Repository Pattern**: Persistence must be abstracted behind a trait.

## Testable Requirements
### Domain Logic
1. **Requirement 1**: Given overlapping slots, the system must correctly identify the intersection and the number of users present.
2. **Requirement 2**: Given multiple slots with different attendance, the system must rank the one with the highest attendance first.
3. **Requirement 3**: Given slots with identical attendance, the system must rank the earliest slot first.
4. **Requirement 4**: The system must return exactly N slots if available, or fewer if not enough overlaps exist.

### Persistence
5. **Requirement 5**: Given a set of user availabilities, saving them to the repository and then loading them back must return an identical set of data.
6. **Requirement 6**: If the database file does not exist, the repository must return an empty list of availabilities.
7. **Requirement 7**: The repository must handle malformed lines in the database file by skipping them without crashing.
