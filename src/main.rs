use crate::email::Email;
use crate::serienstream::{Account, Language, Series};
use clap::{App, Arg};
use rand::distributions::Alphanumeric;
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use std::fs::{create_dir, read_to_string, File, OpenOptions};
use std::io::Write;
use std::process::{exit, Command};
use std::str::FromStr;

mod email;
mod proxy;
mod serienstream;

fn main() {
    let matches = App::new("Serienstream Downloader")
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(
            Arg::with_name("url")
                .long("url")
                .short("u")
                .help("Specifies a source via url")
                .takes_value(true)
                .conflicts_with("name")
                .conflicts_with("id"),
        )
        .arg(
            Arg::with_name("name")
                .long("name")
                .short("n")
                .help("Specifies a source via name")
                .takes_value(true)
                .conflicts_with("id")
                .conflicts_with("url"),
        )
        .arg(
            Arg::with_name("id")
                .long("id")
                .short("i")
                .help("Specifies a source via id")
                .takes_value(true)
                .conflicts_with("name")
                .conflicts_with("url"),
        )
        .arg(
            Arg::with_name("output")
                .long("output")
                .short("o")
                .help("Specifies a folder to save")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("german")
                .long("german")
                .short("g")
                .help("Only downloads german streams")
                .conflicts_with("gersub")
                .conflicts_with("english"),
        )
        .arg(
            Arg::with_name("gersub")
                .long("gersub")
                .short("s")
                .help("Only downloads streams with german subtitles")
                .conflicts_with("german")
                .conflicts_with("english"),
        )
        .arg(
            Arg::with_name("english")
                .long("english")
                .short("e")
                .help("Only downloads english streams")
                .conflicts_with("german")
                .conflicts_with("gersub"),
        )
        .arg(
            Arg::with_name("season")
                .long("season")
                .help("Downloads whole season, --season 1")
                .conflicts_with("episode")
                .conflicts_with("series")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("episode")
                .long("episode")
                .help("Downloads 1 episode, --episode 1,0")
                .conflicts_with("series")
                .conflicts_with("series")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("series")
                .long("series")
                .help("Downloads whole series")
                .conflicts_with("episode")
                .conflicts_with("season"),
        )
        .arg(
            Arg::with_name("generate")
                .long("generate")
                .help("Generates Accounts")
                .takes_value(true),
        )
        .get_matches();

    if matches.is_present("generate") {
        let raw = matches.value_of("generate").unwrap();
        let amount = u32::from_str(raw).unwrap();
        File::create("accounts.txt");
        generate_account(amount);
        exit(0);
    }

    let acclist = read_to_string("accounts.txt");
    match acclist {
        Err(_) => {
            println!("Please add some accounts first (--generate)");
            exit(0);
        }
        Ok(s) => {
            if s.len() < 2 {
                println!("Please add some accounts first (--generate)");
                exit(0);
            }
        }
    }

    let s: Series;
    let lang: Language;
    let output: String;
    let urls: Vec<String>;

    if matches.is_present("url") {
        s = Series::from_url(matches.value_of("url").unwrap());
    } else if matches.is_present("name") {
        s = Series::from_name(matches.value_of("name").unwrap());
    } else if matches.is_present("id") {
        s = Series::from_id(matches.value_of("id").unwrap().parse::<u32>().unwrap());
    } else {
        println!("You need to specify a source");
        exit(0);
    }

    if matches.is_present("german") {
        lang = Language::German;
    } else if matches.is_present("english") {
        lang = Language::English
    } else if matches.is_present("gersub") {
        lang = Language::GermanSubtitles
    } else {
        lang = Language::Unknown
    }

    if matches.is_present("output") {
        output = matches.value_of("output").unwrap().to_string();
    } else {
        output = format!("{}", s.id);
    }
    create_dir(output.clone());

    if matches.is_present("episode") {
        let raw = matches.value_of("episode").unwrap();
        urls = download_episode(s, raw);
    } else if matches.is_present("season") {
        let raw = matches.value_of("season").unwrap();
        urls = download_season(s, raw);
    } else {
        urls = download_series(s);
    }

    println!("[*] Downloading episodes via youtube-dl");
    for url in urls {
        let mut p = Command::new("youtube-dl");
        p.arg(url)
            .arg("--output")
            .arg(format!("{}/%(title)s.%(ext)s", output.clone()))
            .output();
    }
    println!("Everything should be saved in: {}/\nEnjoy!", output)
}

fn random_string(n: usize) -> String {
    thread_rng().sample_iter(&Alphanumeric).take(n).collect()
}

fn generate_account(amount: u32) {
    if amount == 0 {
        return;
    }
    let acc = Account::create(random_string(8), Email::new_from_time(), random_string(8));
    if acc.is_none() {
        generate_account(amount);
        return;
    }
    let acc = acc.unwrap();
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .open("accounts.txt")
        .unwrap();
    write!(
        file,
        "{}",
        format!("\n{}:{}", acc.email.to_string(), acc.password)
    );
    generate_account(amount - 1)
}

fn download_series(s: Series) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    let series_len = s.get_season_count();
    for se in 1..series_len {
        let vec = download_season(s.clone(), format!("{}", se).as_str());
        for vec_entry in vec {
            urls.push(vec_entry);
        }
    }
    urls
}

fn download_season(s: Series, raw: &str) -> Vec<String> {
    let mut urls: Vec<String> = Vec::new();
    let season = u32::from_str(raw).unwrap();
    let season_len = s.get_season(season).get_episode_count();
    for e in 0..season_len {
        let vec = download_episode(s.clone(), format!("{},{}", season, e).as_str());
        urls.push(vec[0].clone());
    }
    urls
}

fn download_episode(s: Series, raw: &str) -> Vec<String> {
    let list_raw = read_to_string("accounts.txt").unwrap();
    let mut list: Vec<&str> = list_raw.split("\n").collect();
    list.shuffle(&mut rand::thread_rng());
    if list[0].len() < 7 {
        println!("[!] Found not valid Account.");
        return download_episode(s, raw);
    }
    let acc = Account::from_str(list[0]);
    let mut urls: Vec<String> = Vec::new();
    let info: Vec<&str> = raw.split(",").collect();
    let season = u32::from_str(info[0]).unwrap();
    let episode = u32::from_str(info[1]).unwrap();
    let url = s
        .get_season(season)
        .get_episode(episode)
        .get_stream_url()
        .get_site_url(acc.clone());
    match url {
        None => download_episode(s, raw),
        Some(url) => {
            urls.push(url);
            urls
        }
    }
}
