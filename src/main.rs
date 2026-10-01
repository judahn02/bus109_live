fn main() {
    // TODO: when this becomes a polling loop, poll every 30s rather than 15s
    // to halve how often the phone's radio wakes up. Predictions rarely change
    // faster than that, and 30s is still plenty of notice for a 5-minute
    // arrival warning. If you change the interval, keep bus109's
    // POOL_IDLE_TIMEOUT (45s) longer than it.
    match bus109::upcoming_buses() {
        Ok(rows) => {
            for [time, left, uncert] in rows.iter() {
                println!("{time} - {left} - {uncert} uncert.");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
