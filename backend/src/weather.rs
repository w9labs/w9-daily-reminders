use chrono::{DateTime, Timelike, Utc};
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WeatherError {
  #[error("request failed: {0}")]
  Request(#[from] reqwest::Error),
  #[error("location resolution failed")]
  Geocoding,
}

#[derive(Debug, Clone, Deserialize)]
struct GeocodeResponse {
  results: Option<Vec<GeocodeResult>>,
}

#[derive(Debug, Clone, Deserialize)]
struct GeocodeResult {
  latitude: f64,
  longitude: f64,
  name: String,
  country: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ForecastResponse {
  current: Option<CurrentWeather>,
  hourly: Option<HourlyForecast>,
  daily: Option<DailyForecast>,
}

#[derive(Debug, Clone, Deserialize)]
struct CurrentWeather {
  temperature_2m: f64,
  apparent_temperature: f64,
  wind_speed_10m: f64,
  weather_code: i32,
}

#[derive(Debug, Clone, Deserialize)]
struct HourlyForecast {
  time: Vec<String>,
  temperature_2m: Vec<f64>,
  apparent_temperature: Vec<f64>,
  wind_speed_10m: Vec<f64>,
  weather_code: Vec<i32>,
}

#[derive(Debug, Clone, Deserialize)]
struct DailyForecast {
  time: Vec<String>,
  temperature_2m_max: Vec<f64>,
  temperature_2m_min: Vec<f64>,
  apparent_temperature_max: Vec<f64>,
  wind_speed_10m_max: Vec<f64>,
  weather_code: Vec<i32>,
}

#[derive(Clone)]
pub struct WeatherClient {
  http: reqwest::Client,
}

impl WeatherClient {
  pub fn new() -> Self {
    Self {
      http: reqwest::Client::new(),
    }
  }

  pub async fn advisory(&self, location: &str) -> Result<String, WeatherError> {
    let (lat, lon, resolved) = self.resolve_location(location).await?;
    let url = format!(
      "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,apparent_temperature,wind_speed_10m,weather_code"
    );
    let resp: ForecastResponse = self.http.get(url).send().await?.error_for_status()?.json().await?;
    let c = resp.current.ok_or(WeatherError::Geocoding)?;
    let mut message = format!(
      "Weather · {resolved}: {temp:.0}°C (feels {feels:.0}°C) · wind {wind:.0} m/s",
      resolved = resolved,
      temp = c.temperature_2m,
      feels = c.apparent_temperature,
      wind = c.wind_speed_10m
    );

    if c.apparent_temperature < 3.0 {
      message.push_str(" · frost risk")
    } else if c.apparent_temperature > 28.0 {
      message.push_str(" · hydrate")
    }

    if c.wind_speed_10m > 10.0 {
      message.push_str(" · high wind, secure umbrella")
    }

    if matches!(c.weather_code, 51..=67 | 80..=82) {
      message.push_str(" · bring umbrella")
    }

    Ok(message)
  }

  pub async fn day_forecast_4h(&self, location: &str, target_date: DateTime<Utc>) -> Result<String, WeatherError> {
    let (lat, lon, resolved) = self.resolve_location(location).await?;
    let start = target_date.format("%Y-%m-%d").to_string();
    let end = (target_date + chrono::Duration::days(1)).format("%Y-%m-%d").to_string();
    
    let url = format!(
      "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&hourly=temperature_2m,apparent_temperature,wind_speed_10m,weather_code&start_date={start}&end_date={end}"
    );
    let resp: ForecastResponse = self.http.get(url).send().await?.error_for_status()?.json().await?;
    
    let hourly = resp.hourly.ok_or(WeatherError::Geocoding)?;
    let mut forecasts = Vec::new();
    
    // Get forecasts at 00:00, 04:00, 08:00, 12:00, 16:00, 20:00 (6 forecasts)
    let target_hours = [0, 4, 8, 12, 16, 20];
    
    for (idx, time_str) in hourly.time.iter().enumerate() {
      if let Ok(dt) = DateTime::parse_from_rfc3339(time_str) {
        let hour = dt.hour();
        let dt_utc = dt.with_timezone(&Utc);
        if target_hours.contains(&hour) && dt_utc.date_naive() == target_date.date_naive() {
          let temp = hourly.temperature_2m.get(idx).copied().unwrap_or(0.0);
          let feels = hourly.apparent_temperature.get(idx).copied().unwrap_or(0.0);
          let wind = hourly.wind_speed_10m.get(idx).copied().unwrap_or(0.0);
          let code = hourly.weather_code.get(idx).copied().unwrap_or(0);
          
          let mut notes = Vec::new();
          if feels < 3.0 {
            notes.push("coat");
          } else if feels > 28.0 {
            notes.push("hydrate");
          }
          if wind > 10.0 {
            notes.push("wind");
          }
          if matches!(code, 51..=67 | 80..=82) {
            notes.push("umbrella");
          }
          
          let note_str = if notes.is_empty() {
            String::new()
          } else {
            format!(" ({})", notes.join(", "))
          };
          
          forecasts.push(format!(
            "{:02}:00 · {:.0}°C (feels {:.0}°C){note}",
            hour, temp, feels, note = note_str
          ));
        }
      }
    }
    
    Ok(format!("Weather · {} · {}", resolved, forecasts.join(" · ")))
  }

  pub async fn week_forecast(&self, location: &str, week_start: DateTime<Utc>) -> Result<String, WeatherError> {
    let (lat, lon, resolved) = self.resolve_location(location).await?;
    let start = week_start.format("%Y-%m-%d").to_string();
    let end = (week_start + chrono::Duration::days(7)).format("%Y-%m-%d").to_string();
    
    let url = format!(
      "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&daily=temperature_2m_max,temperature_2m_min,apparent_temperature_max,wind_speed_10m_max,weather_code&start_date={start}&end_date={end}"
    );
    let resp: ForecastResponse = self.http.get(url).send().await?.error_for_status()?.json().await?;
    
    let daily = resp.daily.ok_or(WeatherError::Geocoding)?;
    let mut day_notes = Vec::new();
    let day_names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    
    for idx in 0..daily.time.len().min(7) {
      
      let max_temp = daily.apparent_temperature_max.get(idx).copied().unwrap_or(0.0);
      let min_temp = daily.temperature_2m_min.get(idx).copied().unwrap_or(0.0);
      let wind = daily.wind_speed_10m_max.get(idx).copied().unwrap_or(0.0);
      let code = daily.weather_code.get(idx).copied().unwrap_or(0);
      
      let mut notes = Vec::new();
      if min_temp < 3.0 {
        notes.push("coat");
      } else if max_temp > 28.0 {
        notes.push("hydrate");
      }
      if wind > 10.0 {
        notes.push("wind");
      }
      if matches!(code, 51..=67 | 80..=82) {
        notes.push("umbrella");
      }
      
      if !notes.is_empty() {
        let day_name = day_names.get(idx).copied().unwrap_or("?");
        day_notes.push(format!("{}: {}", day_name, notes.join(", ")));
      }
    }
    
    if day_notes.is_empty() {
      Ok(format!("Weather · {} · Week forecast: No special attention needed", resolved))
    } else {
      Ok(format!("Weather · {} · Attention needed: {}", resolved, day_notes.join("; ")))
    }
  }

  async fn resolve_location(&self, location: &str) -> Result<(f64, f64, String), WeatherError> {
    if let Some((lat, lon)) = parse_lat_lon(location) {
      return Ok((lat, lon, format!("{lat},{lon}")))
    }

    let url = format!("https://geocoding-api.open-meteo.com/v1/search?name={}&count=1", urlencoding::encode(location));
    let resp: GeocodeResponse = self.http.get(url).send().await?.error_for_status()?.json().await?;
    let result = resp.results.and_then(|mut list| list.pop()).ok_or(WeatherError::Geocoding)?;
    let label = match result.country {
      Some(country) => format!("{}, {}", result.name, country),
      None => result.name,
    };
    Ok((result.latitude, result.longitude, label))
  }
}

fn parse_lat_lon(input: &str) -> Option<(f64, f64)> {
  let mut parts = input.split(',');
  let lat = parts.next()?.trim().parse().ok()?;
  let lon = parts.next()?.trim().parse().ok()?;
  Some((lat, lon))
}
