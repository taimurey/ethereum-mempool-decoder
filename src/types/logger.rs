use chrono::Local;
use colored::Colorize;
use ethabi::Token;

pub fn token_to_string(token: &Token) -> String {
    match token {
        Token::Address(address) => format!("0x{}", hex::encode(address)),
        Token::Uint(uint) => uint.to_string(),
        Token::Int(int) => int.to_string(),
        Token::Bool(b) => b.to_string(),
        Token::String(s) => s.clone(),
        Token::Bytes(bytes) => format!("0x{}", hex::encode(bytes)),
        Token::FixedBytes(bytes) => format!("0x{}", hex::encode(bytes)),
        Token::Array(arr) => format!(
            "[{}]",
            arr.iter()
                .map(token_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Token::FixedArray(arr) => format!(
            "[{}]",
            arr.iter()
                .map(token_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Token::Tuple(tuple) => format!(
            "({})",
            tuple
                .iter()
                .map(token_to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

pub fn log_decoded_input(function_name: &str, input_data: &str) {
    let timestamp = Local::now().format("%H:%M:%S%.3f").to_string();
    let header = format!("{} ⮞ Decoded input for {}:", timestamp, function_name).bright_blue();

    println!("\n{}", header);

    if function_name == "multicall" {
        println!("  {}", "multicall".yellow());
        let calls: Vec<&str> = input_data.split("], [").collect();
        for (i, call) in calls.iter().enumerate() {
            println!("    Call {}:", i + 1);
            let parts: Vec<&str> = call
                .trim_matches(|c| c == '[' || c == ']' || c == '(' || c == ')')
                .split(", ")
                .collect();
            if parts.len() >= 2 {
                println!("      {}: {}", "target".yellow(), parts[0].bright_green());
                println!(
                    "      {}: {}",
                    "callData".yellow(),
                    format!("0x{}...", &parts[1][2..8]).bright_green()
                );
            }
        }
    } else if let Some((param_type, value)) = input_data.split_once(' ') {
        let formatted_type = format!("{}", param_type).yellow();
        println!("  {}", formatted_type);

        if param_type.starts_with("(") {
            // For tuples, split the content and display each item on a new line
            let items = value.trim_matches(|c| c == '(' || c == ')').split(',');
            let types = param_type.trim_matches(|c| c == '(' || c == ')').split(',');

            for (item_type, item_value) in types.zip(items) {
                let formatted_item_type = format!("    {:<15}", item_type.trim()).yellow();
                let formatted_item_value = item_value.trim().bright_green();
                println!("  {} {}", formatted_item_type, formatted_item_value);
            }
        } else if param_type.starts_with("bytes") {
            let formatted_value = format!("0x{}...", &value[2..8]).bright_green();
            println!("    {}", formatted_value);
        } else {
            let formatted_value = value.bright_green();
            println!("    {}", formatted_value);
        }
    } else {
        println!("  {}", input_data.bright_green());
    }

    println!();
}
