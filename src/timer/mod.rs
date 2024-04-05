use chrono::{NaiveTime, Timelike};

const SECONDS_IN_DAY: u32 = 86_400;


#[derive(Clone)]
pub struct TimeRange {
    pub start: u32,
    pub end: u32
}


pub fn parse_time(input: &str) -> Result<TimeRange, Box<dyn std::error::Error>> {

    let mut sp = input.split('-');

    let start = NaiveTime::parse_from_str(sp.next().ok_or("")?, "%H:%M:%S")?.num_seconds_from_midnight();
    let end = NaiveTime::parse_from_str(sp.next().ok_or("")?, "%H:%M:%S")?.num_seconds_from_midnight();

    let time_range = TimeRange {
        start,
        end
    };

    Ok(time_range)
}

pub trait TimedDownload {
    fn should_coutinue(&self) -> bool;
}

impl TimedDownload for Option<TimeRange> {
    fn should_coutinue(&self) -> bool {
        match self {
            Some(timer) => {
                if timer.now_in_range() {
                    return true;
                } else {
                    return false;
                }
            }
            None => true
        }
    }
}

impl TimeRange {

    fn now_in_range(&self) -> bool {
        let now = chrono::Local::now().time().num_seconds_from_midnight();
        self.is_in_time_range(now)
    }

    fn is_in_time_range(&self, mut now: u32) -> bool {
        let mut time_end = self.end;
        if self.start > self.end {
            if time_end > now {
                now += SECONDS_IN_DAY;
            }
            time_end += SECONDS_IN_DAY;
        }
        let is_past_start = self.start < now;
        let is_before_end = time_end > now;
        if is_past_start && is_before_end {
            true
        } else {
            false
        }
    }
}


#[cfg(test)]
mod timer_tests {
    use super::*;

    struct HS(pub u32, pub u32, pub u32);
    fn helper(start: HS, end: HS, now: HS) -> bool {
        let start = NaiveTime::from_hms_opt(start.0, start.1, start.2).unwrap().num_seconds_from_midnight();
        let end = NaiveTime::from_hms_opt(end.0, end.1, end.2).unwrap().num_seconds_from_midnight();
        let now = NaiveTime::from_hms_opt(now.0, now.1, now.2).unwrap().num_seconds_from_midnight();

        let time_range = TimeRange {
            start,
            end
        };

        time_range.is_in_time_range(now)
    }

    #[test]
    fn timer_logic() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(true, helper(HS(2, 0, 0), HS(5, 0, 0), HS(3, 0, 0)));
        assert_eq!(false, helper(HS(2, 0, 0), HS(5, 0, 0), HS(1, 0, 0)));
        assert_eq!(false, helper(HS(2, 0, 0), HS(5, 0, 0), HS(6, 0, 0)));

        assert_eq!(true, helper(HS(21, 0, 0), HS(3, 0, 0), HS(2, 0, 0)));
        assert_eq!(true, helper(HS(21, 0, 0), HS(3, 0, 0), HS(23, 0, 0)));
        assert_eq!(false, helper(HS(21, 0, 0), HS(3, 0, 0), HS(19, 0, 0)));
        assert_eq!(false, helper(HS(21, 0, 0), HS(3, 0, 0), HS(6, 0, 0)));

        Ok(())
    }
}





