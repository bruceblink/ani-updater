pub mod utils;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use crate::add;
    use chrono::{NaiveDate, TimeZone, Utc};

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
    #[test]
    fn tests() {
        let date = NaiveDate::parse_from_str("2025/06/17", "%Y/%m/%d").unwrap();
        let dt = date.and_hms_opt(0, 0, 0).unwrap();
        let dt_utc = Utc.from_utc_datetime(&dt);
        let ts = dt_utc.timestamp_millis();
        assert_eq!(ts, 1750118400000);
    }
}
