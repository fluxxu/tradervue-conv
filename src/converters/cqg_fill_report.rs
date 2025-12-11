/// Convert CQG Fill Report to TraderVue CSV format
///
/// Expected output columns: Date, Time, Symbol, Quantity, Price, Side
pub fn convert(rows: Vec<Vec<String>>) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    if rows.len() < 2 {
        return Err("File must contain at least 2 rows (date line and header)".into());
    }

    let mut result = Vec::new();

    // Add CSV header
    result.push(vec![
        "Date".to_string(),
        "Time".to_string(),
        "Symbol".to_string(),
        "Quantity".to_string(),
        "Price".to_string(),
        "Side".to_string(),
    ]);

    // Parse date from first line
    // Expected: "Fills reported as of 12/10/25 8:20:27 PM for the following accounts: ac214461 (214461)"
    let date = parse_date_from_first_line(&rows[0][0])?;

    // Parse header row to find column indices
    let header_indices = parse_header_row(&rows[1])?;

    // Determine where data rows end (before empty line and disclaimer)
    let data_end_idx = find_data_end_index(&rows);

    // Process data rows (starting from row 2, ending before the empty/disclaimer lines)
    for row in rows.iter().skip(2).take(data_end_idx.saturating_sub(2)) {
        if row.is_empty() || row.iter().all(|s| s.trim().is_empty()) {
            continue; // Skip any empty rows within data
        }

        let csv_row = convert_row(row, &date, &header_indices)?;
        result.push(csv_row);
    }

    Ok(result)
}

/// Find the index where data rows end (before empty line and disclaimer)
fn find_data_end_index(rows: &[Vec<String>]) -> usize {
    // Look for the last row that starts with "Disclaimer"
    for (i, row) in rows.iter().enumerate().rev() {
        if !row.is_empty() && row[0].trim().starts_with("Disclaimer") {
            // Return the index 2 rows before disclaimer (data ends, then empty, then disclaimer)
            return i.saturating_sub(1);
        }
    }
    // If no disclaimer found, process all rows
    rows.len()
}

/// Parse the date from the first line and convert to MM/DD/YYYY format
fn parse_date_from_first_line(line: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Extract date portion from "Fills reported as of 12/10/25 8:20:27 PM..."
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Find "of" and get the next element which should be the date
    let date_str = parts
        .iter()
        .position(|&s| s == "of")
        .and_then(|i| parts.get(i + 1))
        .ok_or("Could not find date in first line")?;

    // Convert MM/DD/YY to MM/DD/YYYY
    let date_parts: Vec<&str> = date_str.split('/').collect();
    if date_parts.len() != 3 {
        return Err(format!("Invalid date format: {}", date_str).into());
    }

    let month = date_parts[0];
    let day = date_parts[1];
    let year = date_parts[2];

    // Convert 2-digit year to 4-digit year (assuming 20xx)
    let full_year = if year.len() == 2 {
        format!("20{}", year)
    } else {
        year.to_string()
    };

    Ok(format!("{}/{}/{}", month, day, full_year))
}

/// Parse header row to find column indices
struct HeaderIndices {
    time_idx: usize,
    symbol_idx: usize,
    buy_idx: usize,
    sell_idx: usize,
    fill_price_idx: usize,
}

fn parse_header_row(header_row: &[String]) -> Result<HeaderIndices, Box<dyn std::error::Error>> {
    let mut time_idx = None;
    let mut symbol_idx = None;
    let mut buy_idx = None;
    let mut sell_idx = None;
    let mut fill_price_idx = None;

    for (i, col) in header_row.iter().enumerate() {
        let col_trimmed = col.trim();

        if col_trimmed == "Time" {
            time_idx = Some(i);
        } else if col_trimmed == "Symbol" {
            symbol_idx = Some(i);
        } else if col_trimmed.starts_with("B (") {
            buy_idx = Some(i);
        } else if col_trimmed.starts_with("S (") {
            sell_idx = Some(i);
        } else if col_trimmed == "Fill P" {
            fill_price_idx = Some(i);
        }
    }

    Ok(HeaderIndices {
        time_idx: time_idx.ok_or("Could not find 'Time' column")?,
        symbol_idx: symbol_idx.ok_or("Could not find 'Symbol' column")?,
        buy_idx: buy_idx.ok_or("Could not find 'B (...)' column")?,
        sell_idx: sell_idx.ok_or("Could not find 'S (...)' column")?,
        fill_price_idx: fill_price_idx.ok_or("Could not find 'Fill P' column")?,
    })
}

/// Convert a single data row
fn convert_row(
    row: &[String],
    date: &str,
    indices: &HeaderIndices,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Extract time and convert to HH:mm:ss format
    let time = convert_time(row.get(indices.time_idx).map(|s| s.as_str()).unwrap_or(""))?;

    // Extract symbol
    let symbol = row
        .get(indices.symbol_idx)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    // Determine side and quantity based on B/S columns
    let buy_qty = row
        .get(indices.buy_idx)
        .map(|s| s.trim())
        .unwrap_or("")
        .to_string();
    let sell_qty = row
        .get(indices.sell_idx)
        .map(|s| s.trim())
        .unwrap_or("")
        .to_string();

    let (side, quantity) = if !buy_qty.is_empty() && buy_qty != "0" {
        ("Buy".to_string(), buy_qty)
    } else if !sell_qty.is_empty() && sell_qty != "0" {
        ("Sell".to_string(), sell_qty)
    } else {
        return Err("Could not determine side: both B and S columns are empty or zero".into());
    };

    // Extract fill price
    let price = row
        .get(indices.fill_price_idx)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    Ok(vec![
        date.to_string(),
        time,
        symbol,
        quantity,
        price,
        side,
    ])
}

/// Convert time to HH:mm:ss format
fn convert_time(time_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let time_str = time_str.trim();

    // If already in HH:mm:ss format, return as is
    if time_str.matches(':').count() == 2 {
        return Ok(time_str.to_string());
    }

    // Handle various time formats
    // Examples: "9:30:15 AM", "14:30:15", etc.
    let parts: Vec<&str> = time_str.split_whitespace().collect();

    if parts.is_empty() {
        return Err("Empty time string".into());
    }

    let time_part = parts[0];
    let time_components: Vec<&str> = time_part.split(':').collect();

    if time_components.len() < 2 {
        return Err(format!("Invalid time format: {}", time_str).into());
    }

    let mut hour: i32 = time_components[0].parse()?;
    let minute: i32 = time_components[1].parse()?;
    let second: i32 = if time_components.len() >= 3 {
        time_components[2].parse()?
    } else {
        0
    };

    // Handle AM/PM if present
    if parts.len() > 1 {
        let meridiem = parts[1].to_uppercase();
        if meridiem == "PM" && hour != 12 {
            hour += 12;
        } else if meridiem == "AM" && hour == 12 {
            hour = 0;
        }
    }

    Ok(format!("{:02}:{:02}:{:02}", hour, minute, second))
}
