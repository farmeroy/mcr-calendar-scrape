use chrono::{DateTime, Local};
use reqwest::{self, blocking::get};
use scraper::{self, Html, Selector};

struct HouseCheckIn {
    check_ins: Vec<DateTime<Local>>,
    check_outs: Vec<DateTime<Local>>,
}

fn main() {
    let response =
        get("https://www.mendocinovacations.com/houses/asst-lightkeepers-house/calendar");

    let data = response.unwrap().text().unwrap();

    let calendar_selector = Selector::parse("div.calendar-container").unwrap();
    let document = Html::parse_document(&data);
    let calendars = document.select(&calendar_selector);

    for calendar in calendars {
        let month = calendar
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .first_child()
            .unwrap()
            .value();
        println!("{:?}", month);
        let fragment = Html::parse_fragment(&calendar.html());
        let checkout_selector = Selector::parse("div.calendar-checkout").unwrap();
        let dates = fragment.select(&checkout_selector);
        for date in dates {
            let check_out_date = date.next_sibling().unwrap().first_child().unwrap().value();
            println!("{:?}", check_out_date);
        }
    }
}
