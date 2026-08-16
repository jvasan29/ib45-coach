use chrono::{DateTime, Utc};

pub fn core_points(tok: &str, ee: &str) -> i64 {
    let grade = |value: &str| match value.trim().to_uppercase().as_str() {
        "A" => Some(0usize),
        "B" => Some(1usize),
        "C" => Some(2usize),
        "D" => Some(3usize),
        "E" => Some(4usize),
        _ => None,
    };
    let matrix = [
        [3, 3, 2, 2, 0],
        [3, 2, 2, 1, 0],
        [2, 2, 1, 0, 0],
        [2, 1, 0, 0, 0],
        [0, 0, 0, 0, 0],
    ];
    match (grade(ee), grade(tok)) {
        (Some(ee_index), Some(tok_index)) => matrix[ee_index][tok_index],
        _ => 0,
    }
}

pub fn percent_to_provisional_grade(percentage: f64) -> i64 {
    match percentage {
        value if value >= 80.0 => 7,
        value if value >= 70.0 => 6,
        value if value >= 60.0 => 5,
        value if value >= 50.0 => 4,
        value if value >= 40.0 => 3,
        value if value >= 30.0 => 2,
        _ => 1,
    }
}

pub fn task_priority(
    due_at: &str,
    effort_minutes: i64,
    expected_impact: f64,
    recurring_weakness_count: i64,
) -> f64 {
    let now = Utc::now();
    let due = DateTime::parse_from_rfc3339(due_at)
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(|_| now + chrono::Duration::days(14));
    let hours = (due - now).num_minutes().max(0) as f64 / 60.0;
    let urgency = if hours <= 24.0 {
        10.0
    } else if hours <= 72.0 {
        7.0
    } else if hours <= 168.0 {
        4.0
    } else {
        1.5
    };
    let efficiency = expected_impact.max(0.1) * 60.0 / effort_minutes.max(15) as f64;
    let recurrence = recurring_weakness_count.min(8) as f64 * 0.6;
    ((urgency + efficiency * 4.0 + recurrence) * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_core_matrix_examples_are_correct() {
        assert_eq!(core_points("A", "A"), 3);
        assert_eq!(core_points("C", "B"), 2);
        assert_eq!(core_points("D", "C"), 0);
        assert_eq!(core_points("E", "A"), 0);
    }

    #[test]
    fn provisional_boundaries_are_monotonic() {
        assert_eq!(percent_to_provisional_grade(82.0), 7);
        assert_eq!(percent_to_provisional_grade(63.0), 5);
        assert_eq!(percent_to_provisional_grade(29.0), 1);
    }

    #[test]
    fn short_high_impact_work_scores_above_long_low_impact_work() {
        let due = (Utc::now() + chrono::Duration::days(2)).to_rfc3339();
        assert!(task_priority(&due, 30, 1.2, 3) > task_priority(&due, 180, 0.2, 0));
    }
}
