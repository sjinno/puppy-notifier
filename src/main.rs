use dotenv;
use form_urlencoded::Serializer;
use reqwest::{self, StatusCode};
use rodio;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::io;
use std::thread;
use std::time;

const URL: &str = "https://www.oregonhumane.org/adopt/?type=dogs";
const DETAIL: &str = "https://www.oregonhumane.org/adopt/details/";
const EXCEPTIONS: [&str; 4] = ["Pit", "Bull", "Chihuahua", "Terrier"];
const MAX_AGE: &u8 = &4;
const INTERVAL: &u64 = &60; // Every 30 seconds.
const NUMBER_OF_REQUESTS: &usize = &90;

#[derive(Clone, Default, Eq, PartialEq, Hash)]
struct Dog {
    name: String,
    breed: String,
    age: String,
    url: String,
}

impl fmt::Display for Dog {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}\n{}\n{}\n{}\n",
            self.name, self.breed, self.age, self.url
        )
    }
}

#[derive(Clone, Default, Eq, PartialEq, Hash)]
struct Id(String);

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Environment variables:
    dotenv::dotenv().ok();
    let telegram_bot_token =
        env::var("TELEGRAM_TOKEN").expect("TELEGRAM_TOKEN not found in .env file.");
    let chat_id = env::var("CHAT_ID").expect("CHAT_ID not found in .env file.");
    // Environemnet variables end.

    let res = reqwest::blocking::get(URL)?;
    // println!("Status: {}", res.status());

    match res.status() {
        StatusCode::OK => {
            let body = res.text()?;
            let mut candidates = get_currently_available_dogs(&body);

            // // ============== TESTING PURPOSE ==============
            // candidates.remove(&Id(String::from("201359")));
            // // =============================================

            for (id, dog) in &candidates {
                println!("Id: {}\n{}", id, dog);
            }

            // Main loop.
            let mut count = 0;
            while count != *NUMBER_OF_REQUESTS {
                thread::sleep(time::Duration::from_secs(*INTERVAL));
                get_update(&mut candidates, &telegram_bot_token, &chat_id)
                    .expect("Uh-oh. Something went wrong.");
                count += 1;
            }
        }
        _ => println!("Uh oh, the link may be broken :("),
    }

    Ok(())
}

// Initializes the initial state of all dogs (filtered with EXCEPTIONS and MIN_AGE)
// that are currently available.
fn get_currently_available_dogs(body: &str) -> HashMap<Id, Dog> {
    let fragment = Html::parse_fragment(&body);
    let dog_type_selector = Selector::parse(r#"div[data-ohssb-type="dog"]"#).unwrap();
    let span_selector = Selector::parse(r#"span"#).unwrap();
    let all_dogs = fragment.select(&dog_type_selector);
    let mut candidates = HashMap::<Id, Dog>::new();

    for dog in all_dogs {
        let mut pup = Dog::default();
        let mut id = Id::default();
        let mut exclude = false;
        let doggy = dog.select(&span_selector);
        for d in doggy {
            let field_name = d.value().attr("class");
            match field_name {
                Some("breed") => {
                    if d.inner_html()
                        .split(' ')
                        .into_iter()
                        .any(|b| EXCEPTIONS.contains(&b))
                    {
                        exclude = true;
                        break;
                    }
                    pup.breed = d.inner_html();
                }
                Some("name") => pup.name = d.inner_html(),
                Some("id") => id.0 = d.inner_html(),
                Some("age") => {
                    let d_clone = d.inner_html().clone();
                    let mut split_age = d_clone.split(' ').take(2);
                    let num = split_age.next();
                    let yr = split_age.next();
                    if let Some(n) = num {
                        match yr {
                            Some("years") => {
                                let n: u8 = match n.parse() {
                                    Ok(num) => num,
                                    Err(_) => continue,
                                };
                                if &n > MAX_AGE {
                                    exclude = true;
                                    break;
                                }
                            }
                            _ => (),
                        }
                    }
                    pup.age = d.inner_html();
                }
                _ => (),
            }
        }
        if exclude {
            continue;
        }
        pup.url = format!("{}{}/", DETAIL, id.0);
        candidates.entry(id).or_insert(pup);
    }

    candidates
}

// Sends a request to OHS website to check if there are any updates on new dogs.
// If there are new dogs, add them to both `candiates` and `new_puppies` lists and call
// `send()` to notify users with the details of the new dogs.
// `new_puppies` list will be dropped after every call of this function.
fn get_update(
    candidates: &mut HashMap<Id, Dog>,
    token: &str,
    chat_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut new_puppies = HashMap::<Id, Dog>::new();
    let res = reqwest::blocking::get(URL)?;

    match res.status() {
        StatusCode::OK => {
            let body = res.text()?;
            let fragment = Html::parse_fragment(&body);
            let dog_type_selector = Selector::parse(r#"div[data-ohssb-type="dog"]"#).unwrap();
            let span_selector = Selector::parse(r#"span"#).unwrap();
            let all_dogs = fragment.select(&dog_type_selector);

            for dog in all_dogs {
                let mut pup = Dog::default();
                let mut id = Id::default();
                let mut exclude = false;
                let doggy = dog.select(&span_selector);
                for d in doggy {
                    let field_name = d.value().attr("class");
                    match field_name {
                        Some("breed") => {
                            if d.inner_html()
                                .split(' ')
                                .into_iter()
                                .any(|b| EXCEPTIONS.contains(&b))
                            {
                                exclude = true;
                                break;
                            }
                            pup.breed = d.inner_html();
                        }
                        Some("name") => pup.name = d.inner_html(),
                        Some("id") => {
                            id.0 = d.inner_html();
                            if candidates.contains_key(&id) {
                                exclude = true;
                                break;
                            }
                        }
                        Some("age") => {
                            let d_clone = d.inner_html().clone();
                            let mut split_age = d_clone.split(' ').take(2);
                            let num = split_age.next();
                            let yr = split_age.next();
                            if let Some(n) = num {
                                match yr {
                                    Some("years") => {
                                        let n: u8 = match n.parse() {
                                            Ok(num) => num,
                                            Err(_) => continue,
                                        };
                                        if &n > MAX_AGE {
                                            exclude = true;
                                            break;
                                        }
                                    }
                                    _ => (),
                                }
                            }
                            pup.age = d.inner_html();
                        }
                        _ => (),
                    }
                }
                if exclude {
                    continue;
                }
                pup.url = format!("{}{}/", DETAIL, id.0);
                new_puppies.entry(id.clone()).or_insert(pup.clone());
                candidates.entry(id).or_insert(pup);
            }
        }
        _ => println!("Uh oh, the link may be broken :("),
    }

    if !new_puppies.is_empty() {
        println!("New puppy/puppies found!");

        let (_stream, stream_handle) = rodio::OutputStream::try_default().unwrap();
        let file = std::fs::File::open("audio/puppy_bark.wav").unwrap();
        let bark = stream_handle.play_once(io::BufReader::new(file)).unwrap();
        bark.set_volume(0.5);

        send(&new_puppies, token, chat_id).expect("Uh-oh. Something went wrong.");
    } else {
        println!("No new puppies posted yet :(");
    }

    Ok(())
}

// Sends Telegram messages if new posts are found.
fn send(
    new_puppies: &HashMap<Id, Dog>,
    telegram_bot_token: &str,
    chat_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    for pup in new_puppies.values() {
        let message = format!(
            "name: {}\n\
                               breed: {}\n\
                               age: {}\n\
                               {}",
            pup.name, pup.breed, pup.age, pup.url
        );
        let parameters = Serializer::new(String::new())
            .append_pair("chat_id", chat_id)
            .append_pair("parse_mode", "Markdown")
            .append_pair("text", &message)
            .finish();
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage?{}",
            telegram_bot_token, parameters
        );
        let client = reqwest::blocking::Client::new();
        client.post(&url).send()?;
    }
    Ok(())
}
