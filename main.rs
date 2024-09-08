use anyhow::Result;
use std::collections::HashMap;
use std::env;
use std::str::FromStr;

use chrono::prelude::*;
use headless_chrome::protocol::cdp::types::Method;
use headless_chrome::{Browser, LaunchOptions};
use reqwest::header;
use serde::Serialize;
use tempdir::TempDir;
use totp_rs::{Algorithm, Secret, TOTP};

use rust_decimal;

#[derive(Serialize, Debug)]
struct Command {
    behavior: String,
    downloadPath: String,
}
impl Method for Command {
    const NAME: &'static str = "Page.setDownloadBehavior";
    type ReturnObject = serde_json::Value;
}

#[derive(Serialize, Debug)]
struct Asset {
    balance: String,
}

async fn fidelity(browser: &Browser) -> Result<HashMap<i64, String>> {
    println!("running fidelity");

    let username = env::var("FIDELITY_USERNAME").unwrap();
    let password = env::var("FIDELITY_PASSWORD").unwrap();
    let totp = env::var("FIDELITY_TOTPSEED").unwrap();

    let tab = browser.new_tab()?;
    let tmp_dir = TempDir::new("lm-amp")?;
    let command = Command {
        behavior: "allow".to_string(),
        downloadPath: tmp_dir.path().as_os_str().to_string_lossy().into(),
    };
    tab.call_method(command)?;

    tab.set_default_timeout(std::time::Duration::from_secs(20));

    tab.enable_stealth_mode();

    println!("fidelity initial nav");
    tab.navigate_to("https://fidelity.com")?;
    // tab.navigate_to("https://digital.fidelity.com/prgw/digital/login/full-page?AuthRedUrl=https://digital.fidelity.com/ftgw/digital/portfolio/summary")?;
    
    std::thread::sleep(std::time::Duration::from_secs(2));
    tab.navigate_to("https://digital.fidelity.com/prgw/digital/login/full-page")?;

    println!("fidelity wait for username");
    tab.wait_for_element("input#dom-username-input")?.click()?;
    tab.type_str(&username)?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    tab.press_key("Tab")?;
    // tab.wait_for_element("input#dom-pswd-input")?.click()?;
    tab.type_str(&password)?;
    // press_key("Enter")?;
    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("fidelity wait for login button");
    tab.wait_for_element("button#dom-login-button")?.click()?;

    println!("fidelity wait for totp field");
    tab.wait_for_element("input#dom-svip-security-code-input")?.click()?;

    println!("enter: totp: {}", totp);
    tab.type_str(&totp)?.press_key("Enter")?;

    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("clicking the totp submit link: {}", Utc::now());
    tab.wait_for_element("button#dom-svip-code-submit-button")?
        .click()?;
    std::thread::sleep(std::time::Duration::from_secs(2));
    
    std::thread::sleep(std::time::Duration::from_secs(2));
    tab.navigate_to("https://digital.fidelity.com/ftgw/digital/portfolio/summary");

    std::thread::sleep(std::time::Duration::from_secs(1000));

    let mut hm = std::collections::HashMap::new();

    Ok(hm)
}

async fn ameriprise(browser: &Browser) -> Result<HashMap<i64, String>> {
    println!("running ameriprise");

    let username = env::var("AMERIPRISE_USERNAME").unwrap();
    let password = env::var("AMERIPRISE_PASSWORD").unwrap();
    let totp = env::var("AMERIPRISE_TOTP").unwrap();

    let tab = browser.new_tab()?;
    let tmp_dir = TempDir::new("lm-amp")?;
    let command = Command {
        behavior: "allow".to_string(),
        downloadPath: tmp_dir.path().as_os_str().to_string_lossy().into(),
    };
    tab.call_method(command)?;

    println!("tempdir = {}", tmp_dir.path().display());

    tab.set_default_timeout(std::time::Duration::from_secs(20));

    tab.navigate_to("https://www.ameriprise.com/client-login")?;

    tab.wait_for_element("input#w-lg-username")?.click()?;
    tab.type_str(&username)?.press_key("Enter")?;

    println!("wait: password");
    tab.wait_for_element("input#w-lg-password")?.click()?;

    println!("enter: password");
    tab.type_str(&password)?.press_key("Enter")?;

    println!("wait: totp");
    tab.wait_for_element("input#w-lg-authcode")?;

    println!("enter: totp: {}", totp);
    tab.type_str(&totp)?.press_key("Enter")?;

    println!("waiting for DL link: {}", Utc::now());
    tab.wait_for_element("button[data-analytics='account-summary-download']")?;

    println!("clicking the DL link: {}", Utc::now());
    tab.wait_for_element("button[data-analytics='account-summary-download']")?
        .click()?;

    println!("end - sleep 5");
    std::thread::sleep(std::time::Duration::from_secs(5));

    // csv

    let mut td = std::fs::read_dir(tmp_dir.path()).unwrap();
    let csv_path = td.next().unwrap().unwrap().path();
    let file = std::fs::File::open(csv_path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);

    let mut hs = std::collections::HashMap::new();
    for result in rdr.records() {
        let record = result?;
        let account_name = record.get(2).unwrap();
        let account_bal = record.get(3).unwrap().replace("$", "").replace(",", "");

        let asset_id = match account_name {
            "INDIVIDUAL" => 101158,
            "AMERIPRISE ONE ACCT" => 101157,
            "IRA_ROLLOVER" => 101156,
            "IRA_ROTH" => 101155,
            "ACT GRW BLDR MOD AGG" => 111678,
            "AMERIPRISE BROKERAGE" => 125989,
            x => {
                println!("unknown account: {}", x);
                todo!()
            }
        };
        hs.insert(asset_id, account_bal);
    }

    Ok(hs)
}

async fn post_balances(balances: &HashMap<i64, String>) -> Result<()> {
    let mut headers = header::HeaderMap::new();
    let lm_token = env::var("LM_TOKEN").unwrap();
    headers.insert(
        "Authorization",
        format!("Bearer {}", lm_token).parse().unwrap(),
    );

    let client = reqwest::ClientBuilder::new().default_headers(headers);
    let client = client.build().unwrap();

    for (asset_id, bal_str) in balances.into_iter() {
        let asset_url = format!("https://dev.lunchmoney.app/v1/assets/{}", asset_id);

        let asset = Asset {
            balance: bal_str.to_string(),
        };

        println!("sending {} -> {:?}", asset_id, asset);

        client
            .put(&asset_url)
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&asset)?)
            .send()
            .await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let headless = env::var("HEADLESS").map(|x| x == "1").unwrap_or(true);
    let crpath = env::var("CHROMIUM_BIN").unwrap();

    println!("here?");

    let pth = std::path::PathBuf::from_str(&crpath);
    let browser = Browser::new(
        LaunchOptions::default_builder()
            .headless(headless)
            .path(Some(pth.unwrap()))
            .build()
            .expect("Could not find chrome-executable"),
    )?;

    println!("here?");

    let balances = ameriprise(&browser).await?;
    // let balances = fidelity(&browser).await?;
    post_balances(&balances).await?;

    let total: rust_decimal::Decimal = balances
        .values()
        .map(|v| rust_decimal::Decimal::from_str(&v).expect("Failed to parse balance as i64"))
        .sum();
    println!("total: {}", total);

    // let balances = fidelity(&browser).await?;
    // post_balances(balances).await?;

    Ok(())
}
