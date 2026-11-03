//! Decodificación incremental de eventos SSE y estado para la reconexión.
use std::time::Duration;
const MAX_EVENT: usize = 1024 * 1024;
pub struct Event {
    pub data: String,
    pub kind: String,
    pub id: String,
}
pub struct Decoder {
    buffer: Vec<u8>,
    data: String,
    kind: String,
    pub last_id: Option<String>,
    pub retry: Duration,
    first: bool,
}
impl Decoder {
    pub fn new(last_id: Option<String>, retry: Duration) -> Self {
        Self {
            buffer: Vec::new(),
            data: String::new(),
            kind: String::new(),
            last_id,
            retry,
            first: true,
        }
    }
    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<Event>, String> {
        let mut events = Vec::new();
        for part in chunk.chunks(16 * 1024) {
            self.buffer.extend_from_slice(part);
            let mut start = 0;
            while let Some(pos) = self.buffer[start..]
                .iter()
                .position(|b| *b == b'\n' || *b == b'\r')
            {
                let end = start + pos;
                if self.buffer[end] == b'\r' && end + 1 == self.buffer.len() {
                    break;
                }
                let next = end
                    + if self.buffer[end] == b'\r' && self.buffer.get(end + 1) == Some(&b'\n') {
                        2
                    } else {
                        1
                    };
                let line = String::from_utf8_lossy(&self.buffer[start..end]).into_owned();
                self.line(&line, &mut events)?;
                start = next;
            }
            self.buffer.drain(..start);
            if self.buffer.len() > MAX_EVENT {
                return Err("Evento SSE demasiado grande (máximo 1 MB)".into());
            }
        }
        Ok(events)
    }
    #[allow(clippy::if_not_else)]
    fn line(&mut self, line: &str, events: &mut Vec<Event>) -> Result<(), String> {
        let line = if self.first {
            self.first = false;
            line.trim_start_matches('\u{feff}')
        } else {
            line
        };
        if line.is_empty() {
            if !self.data.is_empty() {
                self.data.pop();
                events.push(Event {
                    data: std::mem::take(&mut self.data),
                    kind: std::mem::take(&mut self.kind),
                    id: self.last_id.clone().unwrap_or_default(),
                });
            } else {
                self.kind.clear();
            }
            return Ok(());
        }
        if line.starts_with(':') {
            return Ok(());
        }
        let (field, value) = line.split_once(':').map_or((line, ""), |(name, value)| {
            (name, value.strip_prefix(' ').unwrap_or(value))
        });
        match field {
            "data" => {
                if self.data.len() + value.len() + 1 > MAX_EVENT {
                    return Err("Evento SSE demasiado grande (máximo 1 MB)".into());
                }
                self.data.push_str(value);
                self.data.push('\n');
            }
            "event" => value.clone_into(&mut self.kind),
            "id" if !value.contains('\0') => self.last_id = Some(value.to_owned()),
            "retry" if !value.is_empty() && value.bytes().all(|c| c.is_ascii_digit()) => {
                if let Ok(ms) = value.parse::<u64>() {
                    self.retry = Duration::from_millis(ms.clamp(250, 60_000));
                }
            }
            _ => {}
        }
        Ok(())
    }
}
