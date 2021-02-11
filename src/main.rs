use dotenv;
use reqwest;
use scraper::{Html, Selector};
use std::collections::HashSet;
use std::env;
use std::thread;

const URL: &str = "https://www.oregonhumane.org/adopt/?type=dogs";
const DETAIL: &str = "https://www.oregonhumane.org/adopt/details/";
const EXCEPTIONS: [&str; 4] = ["Pit", "Bull", "Chihuahua", "Terrier"];

#[derive(Debug, Eq, PartialEq, Hash)]
struct Dog {
    name: String,
    id: String,
    breed: String,
    age: String,
    url: String,
}

impl Dog {
    fn new() -> Self {
        Self {
            name: String::new(),
            id: String::new(),
            breed: String::new(),
            age: String::new(),
            url: String::new(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    dotenv::dotenv().ok();
    let telegram_bot_token = env::var("TELEGRAM_TOKEN").expect("telegram_bot_token not found.");
    let chat_id = env::var("CHAT_ID").expect("chat_id not found.");

    let res = reqwest::get(URL).await?;
    println!("Status: {}", res.status());

    let body = res.text().await?;
    let fragment = Html::parse_fragment(&body);
    let dog_type_selector = Selector::parse(r#"div[data-ohssb-type="dog"]"#).unwrap();
    let span_selector = Selector::parse(r#"span"#).unwrap();
    let all_dogs = fragment.select(&dog_type_selector);
    let mut candidates = HashSet::<Dog>::new();

    for dog in all_dogs {
        let mut pup = Dog::new();
        let mut exclude = false;
        let doggy = dog.select(&span_selector);
        for d in doggy {
            let cls_name = d.value().attr("class");
            match cls_name {
                Some("breed") => {
                    let d_clone = d.inner_html().clone();
                    let split_breed = d_clone.split(' ').collect::<Vec<_>>();
                    if split_breed.iter().any(|b| EXCEPTIONS.contains(b)) {
                        exclude = true;
                        break;
                    }
                    pup.breed = d_clone;
                }
                Some("name") => pup.name = d.inner_html(),
                Some("id") => pup.id = d.inner_html(),
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
        pup.url = format!("{}{}/", DETAIL, pup.id);
        candidates.insert(pup);
    }

    for dog in &candidates {
        println!("{:?}", dog);
    }

    thread::spawn(move || {
        send(&candidates, &telegram_bot_token, &chat_id).expect("Uh-oh. Something went wrong.");
    })
    .join()
    .unwrap();

    Ok(())
}

#[tokio::main]
async fn send(
    candidates: &HashSet<Dog>,
    telegram_bot_token: &str,
    chat_id: &str,
) -> Result<(), reqwest::Error> {
    let pup = candidates.iter().next().unwrap();
    let message = format!("{}", pup.name);

    let send_text = format!(
        "https://api.telegram.org/bot{}/sendMessage?chat_id={}&parse_mode=Markdown&text={}",
        telegram_bot_token, chat_id, message
    );

    let client = reqwest::Client::new();

    client.post(&send_text).send().await?;

    Ok(())
}
