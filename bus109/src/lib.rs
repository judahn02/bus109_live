use chrono::{DateTime, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;
use std::error::Error;
use std::sync::OnceLock;
use std::time::Duration;

const URL: &str = "https://api-v3.mbta.com/predictions";
/// Upper bound on the whole request (connect, send, and read the body).
const TIMEOUT: Duration = Duration::from_secs(10);
/// How long an unused connection stays open for reuse. Must be longer than
/// the caller's polling interval (currently 30s) or every call reconnects;
/// raise it if the interval goes up.
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(45);

/// One client shared by every call, so a poll can reuse the previous poll's
/// connection instead of redoing the TCP + TLS handshake.
static CLIENT: OnceLock<Client> = OnceLock::new();

fn client() -> Result<&'static Client, reqwest::Error> {
    if let Some(c) = CLIENT.get() {
        return Ok(c);
    }
    let c = Client::builder()
        .timeout(TIMEOUT)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        // reqwest's default sends keepalive probes on idle connections, which
        // wakes a phone's radio after polling stops. The short idle timeout
        // above already closes stale connections, so turn the probes off.
        .tcp_keepalive(None)
        .build()?;
    Ok(CLIENT.get_or_init(|| c))
}

#[derive(Deserialize)]
struct Response {
    data: Vec<Prediction>,
}

#[derive(Deserialize)]
struct Prediction {
    attributes: Attributes,
}

#[derive(Deserialize)]
struct Attributes {
    arrival_time: Option<String>,
    arrival_uncertainty: Option<i64>,
}

/// Upcoming Route 109 buses toward Harvard at Broadway @ Raymond St (stop 5490).
/// Each row is [arrival time (ET), time left, uncertainty or "None"].
pub fn upcoming_buses() -> Result<Box<[[String; 3]]>, Box<dyn Error + Send + Sync>> {
    let resp: Response = client()?
        .get(URL)
        .query(&[
            ("filter[stop]", "5490"),
            ("filter[route]", "109"),
            ("filter[direction_id]", "1"),
            ("sort", "arrival_time"),
        ])
        .send()?
        .error_for_status()?
        .json()?;

    let now = Utc::now();

    let rows: Vec<[String; 3]> = resp
        .data
        .into_iter()
        .filter_map(|p| {
            let a = p.attributes;
            // Skip predictions with no arrival time or an unparseable one.
            let arrival = DateTime::parse_from_rfc3339(&a.arrival_time?).ok()?;

            // The API's timestamps already carry the Eastern offset, so
            // formatting them directly gives ET.
            let time = arrival.format("%-I:%M %p").to_string();

            let mins = (arrival.with_timezone(&Utc) - now).num_seconds() / 60;
            let left = if mins <= 0 { "now".to_string() } else { format!("{mins} min") };

            let uncert = match a.arrival_uncertainty {
                Some(secs) => format!("{} min", (secs + 30) / 60),
                None => "None".to_string(),
            };

            Some([time, left, uncert])
        })
        .collect();

    Ok(rows.into_boxed_slice())
}

