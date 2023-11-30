use std::net::SocketAddr;

use axum::{debug_handler, http::StatusCode, routing::get, Json, Router};
use chrono::NaiveDate;
use futures::future;
use regex::Regex;
use reqwest::{self, Client};
use scraper::{self, Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
struct HouseDates {
    house_name: String,
    check_ins: Vec<NaiveDate>,
    check_outs: Vec<NaiveDate>,
}

#[tokio::main]
async fn main() {
    let app = router().await;

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn router() -> Router {
    Router::new().route("/", get(get_house_dates))
}

#[debug_handler]
async fn get_house_dates() -> Json<Vec<HouseDates>> {
    let links = scrape_house_links().await;
    let houses = scrape_house_dates(links).await;
    Json(houses)
}

async fn scrape_house_links() -> Vec<String> {
    let houses_response = reqwest::get("https://www.mendocinovacations.com/houses");
    let html = houses_response.await.unwrap().text().await.unwrap();
    let data = Html::parse_document(&html);
    let selector = Selector::parse("a").unwrap();

    let mut links: Vec<String> = Vec::new();

    for link in data.select(&selector) {
        if let Some(href) = link.value().attr("href") {
            if href.starts_with("https://www.mendocinovacations.com/houses/") {
                links.push(href.to_string());
            }
        }
    }
    links
}

async fn scrape_house_dates(links: Vec<String>) -> Vec<HouseDates> {
    let client = Client::new();
    let houses = future::join_all(links.into_iter().map(|link| {
        let client = &client;
        async move {
            let link = format!("{link}/calendar");
            let response = client.get(link).send();
            let data = response.await.unwrap().text().await.unwrap();
            let date_regex = Regex::new(r"(?<date>\d{4}-\d{2}-\d{2})").unwrap();

            let house_name_selector = Selector::parse("h1 > a").unwrap();

            let calendar_selector = Selector::parse("div.calendar-container").unwrap();
            let document = Html::parse_document(&data);

            let house_name = document
                .select(&house_name_selector)
                .next()
                .unwrap()
                .first_child()
                .unwrap()
                .value();
            let calendars = document.select(&calendar_selector);
            let mut check_outs: Vec<NaiveDate> = Vec::new();
            let mut check_ins: Vec<NaiveDate> = Vec::new();

            for calendar in calendars {
                let month = calendar
                    .first_child()
                    .unwrap()
                    .next_sibling()
                    .unwrap()
                    .first_child()
                    .unwrap()
                    .value();
                let fragment = Html::parse_fragment(&calendar.html());
                let checkout_selector = Selector::parse("div.calendar-checkout").unwrap();
                let dates = fragment.select(&checkout_selector);
                for date in dates {
                    let check_out_date_url = date
                        .prev_sibling()
                        .unwrap()
                        .value()
                        .as_element()
                        .and_then(|a| a.attr("href"));
                    if let Some(check_out_date) =
                        date_regex.find(check_out_date_url.unwrap_or_default())
                    {
                        check_outs.push(
                            NaiveDate::parse_from_str(check_out_date.as_str(), "%Y-%m-%d").unwrap(),
                        );
                    } else {
                    }
                }
                let fragment = Html::parse_fragment(&calendar.html());
                let checkout_selector = Selector::parse("div.calendar-checkin").unwrap();
                let dates = fragment.select(&checkout_selector);
                for date in dates {
                    if let Some(check_out_div) = date.next_sibling() {
                        if let Some(check_out_date) = check_out_div.first_child() {
                            let mut day =
                                check_out_date.value().as_text().unwrap().text.to_string();
                            if day.len() < 2 {
                                day = format!("0{day}")
                            };

                            let date_string = format!("{} {}", day, month.as_text().unwrap().text);
                            check_ins.push(
                                NaiveDate::parse_from_str(&date_string, "%d %B %Y")
                                    .unwrap_or_default(),
                            )
                        }
                    }
                }
            }
            HouseDates {
                house_name: house_name.as_text().unwrap().text.to_string(),
                check_ins,
                check_outs,
            }
        }
    }))
    .await;
    houses
}

fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
