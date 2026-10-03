use std::collections::BTreeMap;

#[derive(Default)]
#[Service]
pub struct NativeNetworkCsvService;

impl NativeNetworkCsvService {
    const FIRST_BYTE_COUNT: i64 = 0;
    const INCOMING: &'static str = "bytes_in";
    const OUTGOING: &'static str = "bytes_out";

    pub fn parse(&self, text: &str) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        self.rows(text)
    }

    fn rows(&self, text: &str) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        let mut columns: Option<(usize, usize)> = None;
        let mut counters: BTreeMap<i32, (i64, i64)> = BTreeMap::new();
        for line in text.lines() {
            self.row(line, &mut columns, &mut counters);
        }
        self.result(columns, counters)
    }

    fn row(
        &self,
        line: &str,
        columns: &mut Option<(usize, usize)>,
        counters: &mut BTreeMap<i32, (i64, i64)>,
    ) -> () {
        let cells: Vec<String> = self.cells(line);
        let header: Option<(usize, usize)> = self.header(&cells);
        self.replace_header(header, columns, counters);
        self.append(&cells, *columns, counters);
    }

    fn replace_header(
        &self,
        header: Option<(usize, usize)>,
        columns: &mut Option<(usize, usize)>,
        counters: &mut BTreeMap<i32, (i64, i64)>,
    ) -> () {
        if let Some(header) = header {
            *columns = Some(header);
            counters.clear();
        }
    }

    fn header(&self, cells: &[String]) -> Option<(usize, usize)> {
        let incoming: Option<usize> = cells
            .iter()
            .position(|cell: &String| -> bool { cell == Self::INCOMING });
        let outgoing: Option<usize> = cells
            .iter()
            .position(|cell: &String| -> bool { cell == Self::OUTGOING });
        incoming.zip(outgoing)
    }

    fn append(
        &self,
        cells: &[String],
        columns: Option<(usize, usize)>,
        counters: &mut BTreeMap<i32, (i64, i64)>,
    ) -> () {
        let record: Option<(i32, (i64, i64))> = self.record(cells, columns);
        if let Some((identifier, bytes)) = record {
            counters.insert(identifier, bytes);
        }
    }

    fn record(
        &self,
        cells: &[String],
        columns: Option<(usize, usize)>,
    ) -> Option<(i32, (i64, i64))> {
        let identifier: i32 = cells.first()?.rsplit('.').next()?.parse().ok()?;
        let columns: (usize, usize) = columns?;
        let incoming: i64 = self.counter(cells.get(columns.0)?)?;
        let outgoing: i64 = self.counter(cells.get(columns.1)?)?;
        Some((identifier, (incoming, outgoing)))
    }

    fn counter(&self, text: &str) -> Option<i64> {
        text.parse::<i64>()
            .ok()
            .filter(|value: &i64| -> bool { *value >= Self::FIRST_BYTE_COUNT })
    }

    fn cells(&self, line: &str) -> Vec<String> {
        let mut cells: Vec<String> = Vec::new();
        let mut cell: String = String::new();
        let mut quoted: bool = false;
        for character in line.chars() {
            let delimiter: bool = [character == ',', !quoted]
                .into_iter()
                .all(std::convert::identity);
            self.character(character, &mut quoted, &mut cell, (&mut cells, delimiter));
        }
        cells.push(cell.trim().to_owned());
        cells
    }

    fn character(
        &self,
        character: char,
        quoted: &mut bool,
        cell: &mut String,
        result: (&mut Vec<String>, bool),
    ) -> () {
        match (character, result.1) {
            ('"', _) => *quoted = !*quoted,
            (_, true) => {
                result.0.push(cell.trim().to_owned());
                cell.clear();
            }
            _ => cell.push(character),
        }
    }

    fn result(
        &self,
        columns: Option<(usize, usize)>,
        counters: BTreeMap<i32, (i64, i64)>,
    ) -> Result<BTreeMap<i32, (i64, i64)>, String> {
        if columns.is_none() {
            return Err(
                "Network counters unavailable: nettop returned no byte-counter header.".to_owned(),
            );
        }
        Ok(counters)
    }
}
