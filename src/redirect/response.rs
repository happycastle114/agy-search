use crate::error::AgyError;

const STATUS_SENTINEL: &[u8] = b"\nAGY_REDIRECT_META:";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HttpStatus {
    Informational,
    Success,
    Redirect,
    Other,
}

impl HttpStatus {
    const fn parse(value: u16) -> Self {
        match value {
            100..=199 => Self::Informational,
            200..=299 => Self::Success,
            300..=399 => Self::Redirect,
            0..=99 | 400..=u16::MAX => Self::Other,
        }
    }
}

#[derive(Debug)]
pub(super) struct HopResponse {
    pub(super) status: HttpStatus,
    pub(super) location: Option<String>,
}

pub(super) fn parse_hop_response(output: &[u8]) -> Result<HopResponse, AgyError> {
    let marker = output
        .windows(STATUS_SENTINEL.len())
        .rposition(|window| window == STATUS_SENTINEL)
        .ok_or(AgyError::OutputInvalid)?;
    let headers = output.get(..marker).ok_or(AgyError::OutputInvalid)?;
    if headers
        .windows(STATUS_SENTINEL.len())
        .any(|window| window == STATUS_SENTINEL)
    {
        return Err(AgyError::OutputInvalid);
    }
    let metadata = output
        .get(marker + STATUS_SENTINEL.len()..)
        .ok_or(AgyError::OutputInvalid)?;
    let (metadata_status, redirect_count) = parse_metadata(metadata)?;
    if redirect_count != 0 {
        return Err(AgyError::OutputInvalid);
    }
    let header_text = std::str::from_utf8(headers).map_err(|_| AgyError::OutputInvalid)?;
    let mut final_response = None;
    for block in header_text
        .split("\r\n\r\n")
        .filter(|block| !block.is_empty())
    {
        let parsed = parse_header_block(block)?;
        match parsed.status {
            HttpStatus::Informational if parsed.location.is_none() => {}
            HttpStatus::Informational => return Err(AgyError::OutputInvalid),
            HttpStatus::Success | HttpStatus::Redirect | HttpStatus::Other => {
                if final_response.replace(parsed).is_some() {
                    return Err(AgyError::OutputInvalid);
                }
            }
        }
    }
    let response = final_response.ok_or(AgyError::OutputInvalid)?;
    if response.status != HttpStatus::parse(metadata_status) {
        return Err(AgyError::OutputInvalid);
    }
    Ok(response)
}

fn parse_metadata(metadata: &[u8]) -> Result<(u16, u32), AgyError> {
    let metadata = std::str::from_utf8(metadata)
        .map_err(|_| AgyError::OutputInvalid)?
        .trim_end();
    let mut fields = metadata.split(':');
    let status = fields
        .next()
        .and_then(|field| field.parse::<u16>().ok())
        .ok_or(AgyError::OutputInvalid)?;
    let redirect_count = fields
        .next()
        .and_then(|field| field.parse::<u32>().ok())
        .ok_or(AgyError::OutputInvalid)?;
    if fields.next().is_some() {
        return Err(AgyError::OutputInvalid);
    }
    Ok((status, redirect_count))
}

fn parse_header_block(block: &str) -> Result<HopResponse, AgyError> {
    let mut lines = block.split("\r\n");
    let status = lines
        .next()
        .and_then(|line| line.strip_prefix("HTTP/"))
        .and_then(|line| line.split_once(char::is_whitespace))
        .filter(|(version, _)| {
            !version.is_empty()
                && version
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || byte == b'.')
        })
        .and_then(|(_, remainder)| remainder.split_ascii_whitespace().next())
        .and_then(|value| value.parse::<u16>().ok())
        .map(HttpStatus::parse)
        .ok_or(AgyError::OutputInvalid)?;
    let mut location = None;
    for line in lines {
        let (name, value) = line.split_once(':').ok_or(AgyError::OutputInvalid)?;
        if name.eq_ignore_ascii_case("location") {
            let value = value.trim();
            if value.is_empty() || location.replace(value.to_owned()).is_some() {
                return Err(AgyError::OutputInvalid);
            }
        }
    }
    Ok(HopResponse { status, location })
}
