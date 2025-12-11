pub mod cqg_fill_report;

use std::path::Path;
use calamine::{Data, ExcelDateTime, Reader, Xlsx, open_workbook};
use chrono::NaiveDateTime;

/// Convert Excel time value (fraction of day) to HH:MM:SS format
fn excel_datetime_to_string(dt: &ExcelDateTime) -> String {
    let naive_dt: NaiveDateTime = dt.as_datetime().expect("valid datetime");
    naive_dt.format("%H:%M:%S").to_string()
}

/// Parse XLSX file into Vec<Vec<String>>
pub fn parse_xlsx(path: &Path) -> Result<Vec<Vec<String>>, Box<dyn std::error::Error>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;

    // Get the first worksheet
    let sheet_name = workbook
        .sheet_names()
        .first()
        .ok_or("No worksheets found")?
        .clone();

    let range = workbook
        .worksheet_range(&sheet_name)?;

    // Convert to Vec<Vec<String>>
    let rows = range
        .rows()
        .map(|row| {
            row.iter()
                .map(|cell| {
                    match cell {
                        Data::DateTime(dt) => {
                            excel_datetime_to_string(dt)
                        }
                        _ => cell.to_string()
                    }
                })
                .collect()
        })
        .collect();

    Ok(rows)
}

/// Write Vec<Vec<String>> to CSV file
pub fn write_csv(path: &Path, rows: Vec<Vec<String>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut writer = csv::Writer::from_path(path)?;

    for row in rows {
        writer.write_record(&row)?;
    }

    writer.flush()?;
    Ok(())
}
