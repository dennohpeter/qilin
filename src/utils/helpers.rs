use crate::abigen;
use anyhow::Result;
use ethers::core::types::Bytes;
use ethers::providers::Provider;
use ethers::providers::Ws;
use log;
use std::error::Error;
use std::sync::Arc;
use url::Url;

pub fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>())
}

pub fn hex_to_bytes(hex: &str) -> Result<Bytes, ()> {
    let mut bytes = Vec::new();

    for i in 0..(hex.len() / 2) {
        let res = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16);
        match res {
            Ok(v) => bytes.push(v),
            Err(_) => return Err(()),
        }
    }

    Ok(Bytes::from(bytes))
}

pub async fn connect_to_network(
    ws_url: &str,
    mw_url: &str,
    chain_id: i32,
) -> Result<(Arc<Provider<Ws>>, Url, i32), Box<dyn Error>> {
    let ws_provider = Arc::new(Provider::<Ws>::connect(ws_url).await?);
    let middleware_url = Url::parse(mw_url)?;
    Ok((ws_provider, middleware_url, chain_id))
}

pub async fn generate_abigen(arg: Vec<String>) -> Result<()> {
    let first_arg = if arg.len() > 1 {
        arg[1].clone()
    } else {
        String::from("")
    };

    match first_arg.get(0..1) {
        Some(_) => {
            if first_arg.contains("abigen") {
                abigen::generate_abigen_for_addresses()
                    .await
                    .expect("Failed to generate abigen");
                return Ok(());
            } else {
            }
        }
        None => {
            println!();
        }
    }

    Ok(())
}

pub fn bytes_to_string(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:02x}", byte))
        .collect::<Vec<String>>()
        .join("")
}

pub fn get_selectors(selector: &[&str]) -> Vec<Bytes> {
    selector
        .iter()
        .map(|s| hex_to_bytes(s).expect("Invalid selector"))
        .collect()
}

pub fn decode_revert_bytes(data: &[u8]) -> Result<String, Box<dyn Error>> {
    let bytes = hex::decode(&data[130..])?;
    match std::str::from_utf8(&bytes) {
        Ok(s) => {
            println!("Decoded string: {}", s);
            Ok(s.to_string())
        }
        Err(e) => Err(Box::new(e)),
    }
}
