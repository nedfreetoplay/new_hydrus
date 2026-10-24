
fn create_qualified_index(
    db: &DBBase,
    qualified_table: &str,
    index_suffix: &str,
    columns: &str,
) -> HydrusResult<()> {
    let (schema, table) = qualified_table
        .split_once('.')
        .unwrap_or(("main", qualified_table));

    let index_name = format!("{table}_{index_suffix}_index");
    let qualified_index = if schema == "main" {
        index_name
    } else {
        format!("{schema}.{index_name}")
    };

    db.execute(
        &format!("CREATE INDEX {qualified_index} ON {table} ( {columns} );"),
        &[],
    )?;

    Ok(())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut text = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 0x0F)]));
    }

    text
}
