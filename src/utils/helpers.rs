use ethers::core::types::Bytes;

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

pub fn get_selectors(selector: &[&str]) -> Vec<Bytes> {
    selector
        .iter()
        .map(|s| hex_to_bytes(s).expect("Invalid selector"))
        .collect()
}
