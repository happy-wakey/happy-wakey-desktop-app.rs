use serde::{Deserialize, Serialize};
use url::Url;

use crate::url_safety::is_loopback_host;

const OPEN_METEO_FORECAST_URL: &str = "https://api.open-meteo.com/v1/forecast";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    pub location_name: String,
    pub temperature: f64,
    pub feels_like: f64,
    pub condition: String,
    pub weather_code: i32,
    pub icon: String,
    pub humidity: f64,
    pub wind_speed: f64,
    pub precipitation: f64,
    pub is_day: bool,
    pub observed_at: String,
    pub source: String,
    pub source_url: String,
    pub forecast: Vec<DailyForecast>,
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DailyForecast {
    pub date: String,
    pub weather_code: i32,
    pub condition: String,
    pub high: f64,
    pub low: f64,
    pub precipitation_probability: f64,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    current: OpenMeteoCurrent,
    #[serde(default)]
    daily: OpenMeteoDaily,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoCurrent {
    time: String,
    temperature_2m: f64,
    relative_humidity_2m: f64,
    apparent_temperature: f64,
    precipitation: f64,
    weather_code: i32,
    wind_speed_10m: f64,
    is_day: i32,
}

#[derive(Debug, Default, Deserialize)]
struct OpenMeteoDaily {
    #[serde(default)]
    time: Vec<String>,
    #[serde(default)]
    weather_code: Vec<i32>,
    #[serde(default)]
    temperature_2m_max: Vec<f64>,
    #[serde(default)]
    temperature_2m_min: Vec<f64>,
    #[serde(default)]
    precipitation_probability_max: Vec<Option<f64>>,
}

#[derive(Debug, Deserialize)]
struct OpenWeatherResponse {
    main: MainData,
    #[serde(default)]
    weather: Vec<WeatherInfo>,
    wind: WindData,
}

#[derive(Debug, Deserialize)]
struct MainData {
    temp: f64,
    feels_like: f64,
    humidity: f64,
}

#[derive(Debug, Deserialize)]
struct WeatherInfo {
    description: String,
    icon: String,
    #[serde(default)]
    id: i32,
}

#[derive(Debug, Deserialize)]
struct WindData {
    speed: f64,
}

/// Open-Meteo is the keyless primary provider. OpenWeather remains a fallback
/// when a key is configured, preserving existing deployments during outages.
pub fn fetch_weather(lat: f64, lon: f64, location_name: &str) -> Result<WeatherData, String> {
    validate_coordinates(lat, lon)?;

    match fetch_open_meteo(lat, lon, location_name) {
        Ok(weather) => Ok(weather),
        Err(open_meteo_error) => {
            let has_openweather_key = std::env::var("OPENWEATHER_API_KEY")
                .ok()
                .is_some_and(|key| !key.trim().is_empty());
            if !has_openweather_key {
                return Err(open_meteo_error);
            }

            fetch_openweather(lat, lon, location_name).map_err(|fallback_error| {
                format!(
                    "Open-Meteo failed ({open_meteo_error}); OpenWeather fallback failed ({fallback_error})"
                )
            })
        }
    }
}

fn fetch_open_meteo(lat: f64, lon: f64, location_name: &str) -> Result<WeatherData, String> {
    let endpoint = std::env::var("OPEN_METEO_BASE_URL")
        .unwrap_or_else(|_| OPEN_METEO_FORECAST_URL.to_string());
    let mut url =
        Url::parse(&endpoint).map_err(|error| format!("Invalid Open-Meteo URL: {error}"))?;
    if url.scheme() != "https" && !url.host_str().is_some_and(is_loopback_host) {
        return Err("Open-Meteo endpoint must use HTTPS".into());
    }

    url.query_pairs_mut()
        .append_pair("latitude", &lat.to_string())
        .append_pair("longitude", &lon.to_string())
        .append_pair(
            "current",
            "temperature_2m,relative_humidity_2m,apparent_temperature,precipitation,weather_code,wind_speed_10m,is_day",
        )
        .append_pair(
            "daily",
            "weather_code,temperature_2m_max,temperature_2m_min,precipitation_probability_max",
        )
        .append_pair("temperature_unit", "fahrenheit")
        .append_pair("wind_speed_unit", "mph")
        .append_pair("precipitation_unit", "inch")
        .append_pair("timezone", "auto")
        .append_pair("forecast_days", "5");

    if let Ok(api_key) = std::env::var("OPEN_METEO_API_KEY") {
        if !api_key.trim().is_empty() {
            url.query_pairs_mut().append_pair("apikey", api_key.trim());
        }
    }

    let response: OpenMeteoResponse =
        crate::http::get_json("Open-Meteo", crate::http::shared_client().get(url))?;
    open_meteo_to_weather(response, lat, lon, location_name)
}

fn open_meteo_to_weather(
    response: OpenMeteoResponse,
    lat: f64,
    lon: f64,
    location_name: &str,
) -> Result<WeatherData, String> {
    let current = response.current;
    let numeric_values = [
        current.temperature_2m,
        current.apparent_temperature,
        current.relative_humidity_2m,
        current.wind_speed_10m,
        current.precipitation,
    ];
    if numeric_values.iter().any(|value| !value.is_finite()) {
        return Err("Open-Meteo returned invalid current conditions".into());
    }

    let forecast_len = [
        response.daily.time.len(),
        response.daily.weather_code.len(),
        response.daily.temperature_2m_max.len(),
        response.daily.temperature_2m_min.len(),
    ]
    .into_iter()
    .min()
    .unwrap_or(0)
    .min(5);

    let forecast = (0..forecast_len)
        .filter_map(|index| {
            let high = response.daily.temperature_2m_max[index];
            let low = response.daily.temperature_2m_min[index];
            if !high.is_finite() || !low.is_finite() {
                return None;
            }
            let code = response.daily.weather_code[index];
            Some(DailyForecast {
                date: response.daily.time[index].clone(),
                weather_code: code,
                condition: wmo_condition(code).to_string(),
                high,
                low,
                precipitation_probability: response
                    .daily
                    .precipitation_probability_max
                    .get(index)
                    .and_then(|value| *value)
                    .filter(|value| value.is_finite())
                    .unwrap_or(0.0)
                    .clamp(0.0, 100.0),
            })
        })
        .collect();

    Ok(WeatherData {
        location_name: location_name.to_string(),
        temperature: current.temperature_2m,
        feels_like: current.apparent_temperature,
        condition: wmo_condition(current.weather_code).to_string(),
        weather_code: current.weather_code,
        icon: format!("wmo-{}", current.weather_code),
        humidity: current.relative_humidity_2m.clamp(0.0, 100.0),
        wind_speed: current.wind_speed_10m.max(0.0),
        precipitation: current.precipitation.max(0.0),
        is_day: current.is_day == 1,
        observed_at: current.time,
        source: "Open-Meteo".into(),
        source_url: "https://open-meteo.com/".into(),
        forecast,
        lat,
        lon,
    })
}

fn fetch_openweather(lat: f64, lon: f64, location_name: &str) -> Result<WeatherData, String> {
    let api_key = std::env::var("OPENWEATHER_API_KEY")
        .map_err(|_| "OPENWEATHER_API_KEY is not configured".to_string())?;

    let mut url = Url::parse("https://api.openweathermap.org/data/2.5/weather")
        .map_err(|error| format!("Invalid OpenWeather URL: {error}"))?;
    url.query_pairs_mut()
        .append_pair("lat", &lat.to_string())
        .append_pair("lon", &lon.to_string())
        .append_pair("units", "imperial")
        .append_pair("appid", api_key.trim());

    let response: OpenWeatherResponse =
        crate::http::get_json("OpenWeather", crate::http::shared_client().get(url))?;
    let condition = response
        .weather
        .first()
        .map(|weather| weather.description.clone())
        .unwrap_or_else(|| "Current conditions".into());
    let icon = response
        .weather
        .first()
        .map(|weather| weather.icon.clone())
        .unwrap_or_default();
    let weather_code = response
        .weather
        .first()
        .map(|weather| weather.id)
        .unwrap_or(0);

    Ok(WeatherData {
        location_name: location_name.to_string(),
        temperature: response.main.temp,
        feels_like: response.main.feels_like,
        condition,
        weather_code,
        icon,
        humidity: response.main.humidity.clamp(0.0, 100.0),
        wind_speed: response.wind.speed.max(0.0),
        precipitation: 0.0,
        is_day: true,
        observed_at: chrono::Utc::now().to_rfc3339(),
        source: "OpenWeather".into(),
        source_url: "https://openweathermap.org/".into(),
        forecast: Vec::new(),
        lat,
        lon,
    })
}

fn validate_coordinates(lat: f64, lon: f64) -> Result<(), String> {
    if !lat.is_finite()
        || !lon.is_finite()
        || !(-90.0..=90.0).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
    {
        Err("Invalid weather coordinates".into())
    } else {
        Ok(())
    }
}

fn wmo_condition(code: i32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mostly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51 | 53 | 55 => "Drizzle",
        56 | 57 => "Freezing drizzle",
        61 | 63 | 65 => "Rain",
        66 | 67 => "Freezing rain",
        71 | 73 | 75 => "Snow",
        77 => "Snow grains",
        80..=82 => "Rain showers",
        85 | 86 => "Snow showers",
        95 => "Thunderstorms",
        96 | 99 => "Thunderstorms with hail",
        _ => "Mixed conditions",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> &'static str {
        r#"{
            "current": {
                "time": "2026-07-16T09:15",
                "temperature_2m": 72.4,
                "relative_humidity_2m": 61.0,
                "apparent_temperature": 73.1,
                "precipitation": 0.01,
                "weather_code": 2,
                "wind_speed_10m": 8.2,
                "is_day": 1
            },
            "daily": {
                "time": ["2026-07-16", "2026-07-17"],
                "weather_code": [2, 61],
                "temperature_2m_max": [81.0, 74.0],
                "temperature_2m_min": [65.0, 62.0],
                "precipitation_probability_max": [20, null]
            }
        }"#
    }

    #[test]
    fn parses_open_meteo_current_and_forecast() {
        let response: OpenMeteoResponse = serde_json::from_str(fixture()).expect("parse fixture");
        let weather =
            open_meteo_to_weather(response, 41.88, -87.63, "Chicago").expect("convert weather");

        assert_eq!(weather.location_name, "Chicago");
        assert_eq!(weather.condition, "Partly cloudy");
        assert_eq!(weather.source, "Open-Meteo");
        assert_eq!(weather.forecast.len(), 2);
        assert_eq!(weather.forecast[1].condition, "Rain");
        assert_eq!(weather.forecast[1].precipitation_probability, 0.0);
    }

    #[test]
    fn maps_wmo_weather_codes() {
        assert_eq!(wmo_condition(0), "Clear sky");
        assert_eq!(wmo_condition(48), "Fog");
        assert_eq!(wmo_condition(99), "Thunderstorms with hail");
        assert_eq!(wmo_condition(500), "Mixed conditions");
    }

    #[test]
    fn rejects_invalid_coordinates() {
        assert!(validate_coordinates(91.0, 0.0).is_err());
        assert!(validate_coordinates(0.0, -181.0).is_err());
        assert!(validate_coordinates(f64::NAN, 0.0).is_err());
    }

    #[test]
    #[ignore = "requires live network access"]
    fn open_meteo_live_smoke() {
        let weather =
            fetch_open_meteo(41.8781, -87.6298, "Chicago").expect("live Open-Meteo request");
        assert!(weather.temperature.is_finite());
        assert!(!weather.forecast.is_empty());
        assert_eq!(weather.source, "Open-Meteo");
    }
}
