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
  current: CurrentWeather,
}

#[derive(Debug, Clone, Deserialize)]
struct CurrentWeather {
  temperature_2m: f64,
  apparent_temperature: f64,
  wind_speed_10m: f64,
  weather_code: i32,
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
    let c = resp.current;
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
