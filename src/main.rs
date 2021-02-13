use dotenv;
use reqwest::{self, StatusCode};
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::thread;
use std::time;

const URL: &str = "https://www.oregonhumane.org/adopt/?type=dogs";
const DETAIL: &str = "https://www.oregonhumane.org/adopt/details/";
const EXCEPTIONS: [&str; 4] = ["Pit", "Bull", "Chihuahua", "Terrier"];
const INTERVAL: u64 = 30; // Every 30 seconds.
const NUMBER_OF_REQUESTS: usize = 60;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct Dog {
    name: String,
    breed: String,
    age: String,
    url: String,
}

impl Dog {
    fn new() -> Self {
        Self {
            name: String::new(),
            breed: String::new(),
            age: String::new(),
            url: String::new(),
        }
    }
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

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct Id(String);

impl Id {
    fn new() -> Self {
        Self(String::new())
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    // Environment variables:
    dotenv::dotenv().ok();
    let telegram_bot_token = env::var("TELEGRAM_TOKEN").expect("telegram_bot_token not found.");
    let chat_id = env::var("CHAT_ID").expect("chat_id not found.");
    // Environemnet variables end.

    let res = reqwest::get(URL).await?;
    // println!("Status: {}", res.status());

    match res.status() {
        StatusCode::OK => {
            let body = res.text().await?;
            let fragment = Html::parse_fragment(&body);
            let dog_type_selector = Selector::parse(r#"div[data-ohssb-type="dog"]"#).unwrap();
            let span_selector = Selector::parse(r#"span"#).unwrap();
            let all_dogs = fragment.select(&dog_type_selector);
            let mut candidates = HashMap::<Id, Dog>::new();

            for dog in all_dogs {
                let mut pup = Dog::new();
                let mut id = Id::new();
                let mut exclude = false;
                let doggy = dog.select(&span_selector);
                for d in doggy {
                    let cls_name = d.value().attr("class");
                    match cls_name {
                        Some("breed") => {
                            let d_clone = d.inner_html().clone();
                            let split_breed = d_clone.split(' ').collect::<Vec<_>>();
                            if split_breed.into_iter().any(|b| EXCEPTIONS.contains(&b)) {
                                exclude = true;
                                break;
                            }
                            pup.breed = d.inner_html();
                        }
                        Some("name") => pup.name = d.inner_html(),
                        Some("id") => id.0 = d.inner_html(),
                        Some("age") => {
                            let d_clone = d.inner_html().clone();
                            let split_age = d_clone.split(' ').collect::<Vec<_>>();
                            for (i, s) in split_age.iter().enumerate() {
                                if s == &"years" {
                                    if split_age[i - 1].parse::<u8>().unwrap() > 4 {
                                        exclude = true;
                                        break;
                                    }
                                }
                            }
                            if exclude {
                                break;
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

            for (key, val) in &candidates {
                println!("Id: {}\n{}", key, val);
            }

            let mut count = 0;
            while count != NUMBER_OF_REQUESTS {
                thread::sleep(time::Duration::from_secs(INTERVAL));
                let mut cands = candidates.clone();
                let token = telegram_bot_token.clone();
                let chat = chat_id.clone();
                thread::spawn(move || {
                    get_update(&mut cands, token, chat).expect("Uh-oh. Something went wrong.");
                })
                .join()
                .unwrap();
                count += 1;
            }
        }
        _ => (),
    }

    Ok(())
}

#[tokio::main]
async fn get_update(
    candidates: &mut HashMap<Id, Dog>,
    token: String,
    chat_id: String,
) -> Result<(), reqwest::Error> {
    let mut new_puppies = HashMap::<Id, Dog>::new();
    let res = reqwest::get(URL).await?;

    match res.status() {
        StatusCode::OK => {
            let body = res.text().await?;
            let fragment = Html::parse_fragment(&body);
            let dog_type_selector = Selector::parse(r#"div[data-ohssb-type="dog"]"#).unwrap();
            let span_selector = Selector::parse(r#"span"#).unwrap();
            let all_dogs = fragment.select(&dog_type_selector);

            for dog in all_dogs {
                let mut pup = Dog::new();
                let mut id = Id::new();
                let mut exclude = false;
                let doggy = dog.select(&span_selector);
                for d in doggy {
                    let cls_name = d.value().attr("class");
                    match cls_name {
                        Some("breed") => {
                            let d_clone = d.inner_html().clone();
                            let split_breed = d_clone.split(' ').collect::<Vec<_>>();
                            if split_breed.into_iter().any(|b| EXCEPTIONS.contains(&b)) {
                                exclude = true;
                                break;
                            }
                            pup.breed = d.inner_html();
                        }
                        Some("name") => pup.name = d.inner_html(),
                        Some("id") => {
                            id.0 = d.inner_html();
                            exclude = candidates.contains_key(&id);
                        }
                        Some("age") => {
                            let d_clone = d.inner_html().clone();
                            let split_age = d_clone.split(' ').collect::<Vec<_>>();
                            for (i, s) in split_age.iter().enumerate() {
                                if s == &"years" {
                                    if split_age[i - 1].parse::<u8>().unwrap() > 4 {
                                        exclude = true;
                                        break;
                                    }
                                }
                            }
                            if exclude {
                                break;
                            }
                            pup.age = d.inner_html()
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
        _ => (),
    }

    if !new_puppies.is_empty() {
        thread::spawn(move || {
            send(&new_puppies, token, chat_id).expect("Uh-oh. Something went wrong.");
        })
        .join()
        .unwrap();
    } else {
        println!("No new puppies posted yet :(");
    }

    Ok(())
}

#[tokio::main]
async fn send(
    new_puppies: &HashMap<Id, Dog>,
    telegram_bot_token: String,
    chat_id: String,
) -> Result<(), reqwest::Error> {
    while let Some(pup) = new_puppies.iter().next() {
        let message = format!("{}", pup.1);
        let send_text = format!(
            "https://api.telegram.org/bot{}/sendMessage?chat_id={}&parse_mode=Markdown&text={}",
            telegram_bot_token, chat_id, message
        );
        let client = reqwest::Client::new();
        client.post(&send_text).send().await?;
    }
    Ok(())
}
