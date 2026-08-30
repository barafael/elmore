//! Weather — exercises `Command::FetchText` against a *real* API
//! ([open-meteo.com](https://open-meteo.com), free and key-less), turning the
//! fetched text into a message and parsing it in `update`. Shows the classic
//! loading / error / loaded states — unplug the network and the error path is
//! what you'll see.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

/// Current conditions for Athens, Greece, from the Open-Meteo API.
const ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast?latitude=37.98&longitude=23.73&current_weather=true";

enum Msg {
    /// The user asked us to (re)fetch the forecast.
    Refresh,
    /// The fetch completed. `Ok` carries the report text, `Err` a reason.
    Fetched(Result<String, String>),
}

#[derive(Default)]
struct Model {
    phase: Phase,
}

#[derive(Default, Clone)]
enum Phase {
    /// Nothing fetched yet.
    #[default]
    Idle,
    /// A fetch is in flight.
    Loading,
    Forecast {
        weather: CurrentWeather,
        /// The raw JSON, shown as-is in a `<pre>` so you can see what fetch
        /// actually returned.
        raw: String,
    },
    /// Last fetch (or parse) failed.
    Failed { reason: String },
}

/// The subset of the Open-Meteo response we care about.
#[derive(serde::Deserialize, Clone)]
struct Forecast {
    #[serde(rename = "current_weather")]
    current: CurrentWeather,
}

#[derive(serde::Deserialize, Clone, Copy)]
struct CurrentWeather {
    temperature: f64,
    windspeed: f64,
    weathercode: u16,
}

impl CurrentWeather {
    /// A human label for the WMO weather code.
    fn label(self) -> &'static str {
        match self.weathercode {
            0 => "clear sky",
            1 | 2 => "mostly clear",
            3 => "overcast",
            45 | 48 => "fog",
            51..=57 => "drizzle",
            61..=67 => "rain",
            71..=77 => "snow",
            80..=82 => "rain showers",
            85 | 86 => "snow showers",
            95..=99 => "thunderstorm",
            _ => "unsettled",
        }
    }
}

#[derive(Default)]
struct Weather;

impl App for Weather {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Refresh => {
                model.phase = Phase::Loading;
                Some(Command::FetchText {
                    url: ENDPOINT.to_string(),
                    on_result: Box::new(Msg::Fetched),
                })
            }
            Msg::Fetched(Ok(text)) => {
                // The effect delivered raw text; parsing is plain `update`
                // logic — no JSON inside `view`, ever.
                match serde_json::from_str::<Forecast>(&text) {
                    Ok(f) => {
                        model.phase = Phase::Forecast { weather: f.current, raw: text };
                    }
                    Err(e) => {
                        model.phase = Phase::Failed { reason: format!("couldn't read the forecast: {e}") };
                    }
                }
                Command::none()
            }
            Msg::Fetched(Err(reason)) => {
                model.phase = Phase::Failed { reason };
                Command::none()
            }
        }
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        let body = match &model.phase {
            Phase::Idle => Html::p().text("No forecast yet."),
            Phase::Loading => Html::p().class("loading").text("Loading…"),
            Phase::Forecast { weather, raw } => Html::div().children([
                Html::p().class("now").text(format!(
                    "Athens: {:.1} °C, {}, wind {:.0} km/h",
                    weather.temperature,
                    weather.label(),
                    weather.windspeed,
                )),
                Html::pre().class("raw").text(raw.clone()),
            ]),
            Phase::Failed { reason } => {
                Html::p().class("error").text(format!("Couldn't load: {reason}"))
            }
        };

        Html::div()
            .class("weather")
            .children([
                Html::h1().text("Weather"),
                body,
                Html::button()
                    .text("Refresh")
                    .disabled(matches!(model.phase, Phase::Loading))
                    .on_click(|| Msg::Refresh),
            ])
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Weather>();
}
