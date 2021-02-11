use reqwest;
use scraper::{Html, Selector};
use std::collections::HashMap;

const EXCEPTIONS: [&str; 3] = ["American Pit Bull", "Chihuahua", "Terrier"];

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let res = reqwest::get("https://www.oregonhumane.org/adopt/?type=dogs").await?;

    println!("Status: {}", res.status());

    let body = res.text().await?;

    let fragment = Html::parse_fragment(&body);
    let div_selector = Selector::parse(r#"div[data-ohssb-type="dog"]"#).unwrap();
    let span_selector = Selector::parse(r#"span"#).unwrap();

    let all_dogs = fragment.select(&div_selector);

    // println!("{:?}", fragment.select(&div_selector).next().unwrap());

    for doggy in all_dogs {
        let dog = doggy.select(&span_selector);
        for d in dog {
            let cls_name = d.value().attr("class");
            match cls_name {
                Some("name") | Some("id") | Some("age") | Some("breed") => {
                    println!("{:}", d.inner_html())
                }
                _ => (),
            }
            // println!("{:}", d.value().attr("class").unwrap());
        }
        println!();
        // println!("{:?}", doggy.value());
    }

    // for doggy in fragment.select(&div_selector) {
    //     if let Some(d) = doggy.value().attr("data-ohssb-name") {
    //         println!("{:?}", d);
    //     }
    // }

    // let input = fragment.select(&selector).next().unwrap();
    // assert_eq!(Some("bar"), input.value().attr("value"));
    // println!("{:?}", element.select(&span_slelector));

    Ok(())
}
