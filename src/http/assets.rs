use axum::{
    body::Body,
    extract::Path,
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{ACCEPT, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/web/dist"]
struct DashboardAssets;

pub(super) async fn spa_shell(headers: HeaderMap) -> Response {
    if !prefers_html(&headers) {
        return (StatusCode::NOT_ACCEPTABLE, "expected Accept: text/html").into_response();
    }

    shell_response()
}

pub(super) async fn static_asset(Path(path): Path<String>) -> Response {
    let embedded_path = format!("assets/{path}");
    serve_embedded(&embedded_path)
}

pub(super) fn prefers_html(headers: &HeaderMap) -> bool {
    let Some(raw_accept) = headers.get(ACCEPT).and_then(|value| value.to_str().ok()) else {
        return false;
    };

    let preferences: Vec<_> = raw_accept
        .split(',')
        .enumerate()
        .filter_map(|(index, value)| AcceptPreference::parse(value, index))
        .collect();

    let html = best_match(&preferences, "text", "html");
    let json = best_match(&preferences, "application", "json");

    match (html, json) {
        (Some(html), Some(json)) => html.outranks(&json),
        (Some(_), None) => true,
        _ => false,
    }
}

pub(super) fn shell_response() -> Response {
    serve_embedded("index.html")
}

fn serve_embedded(path: &str) -> Response {
    let Some(asset) = DashboardAssets::get(path) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let mime = from_path(path).first_or_octet_stream();
    let content_type = HeaderValue::from_str(mime.as_ref())
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));

    (
        StatusCode::OK,
        [(CONTENT_TYPE, content_type)],
        Body::from(asset.data),
    )
        .into_response()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AcceptPreference {
    q_millis: u16,
    specificity: u8,
    index: usize,
    type_name: String,
    subtype_name: String,
}

impl AcceptPreference {
    fn parse(raw_value: &str, index: usize) -> Option<Self> {
        let mut parts = raw_value.split(';');
        let media_range = parts.next()?.trim();
        let (type_name, subtype_name) = media_range.split_once('/')?;

        let mut q_millis = 1_000;
        for parameter in parts {
            let parameter = parameter.trim();
            if let Some(q_value) = parameter.strip_prefix("q=") {
                q_millis = parse_q_millis(q_value)?;
            }
        }

        Some(Self {
            q_millis,
            specificity: 0,
            index,
            type_name: type_name.trim().to_ascii_lowercase(),
            subtype_name: subtype_name.trim().to_ascii_lowercase(),
        })
    }

    fn outranks(&self, other: &Self) -> bool {
        self.q_millis > other.q_millis
            || (self.q_millis == other.q_millis
                && (self.specificity > other.specificity
                    || (self.specificity == other.specificity && self.index < other.index)))
    }
}

fn best_match(
    preferences: &[AcceptPreference],
    expected_type: &str,
    expected_subtype: &str,
) -> Option<AcceptPreference> {
    preferences
        .iter()
        .filter_map(|preference| {
            let specificity = match (
                preference.type_name.as_str(),
                preference.subtype_name.as_str(),
                expected_type,
                expected_subtype,
            ) {
                (type_name, subtype_name, expected_type, expected_subtype)
                    if type_name == expected_type && subtype_name == expected_subtype =>
                {
                    2
                }
                (type_name, "*", expected_type, _) if type_name == expected_type => 1,
                ("*", "*", _, _) => 0,
                _ => return None,
            };

            if preference.q_millis == 0 {
                return None;
            }

            Some(AcceptPreference {
                q_millis: preference.q_millis,
                specificity,
                index: preference.index,
                type_name: preference.type_name.clone(),
                subtype_name: preference.subtype_name.clone(),
            })
        })
        .max_by(|left, right| {
            left.q_millis
                .cmp(&right.q_millis)
                .then(left.specificity.cmp(&right.specificity))
                .then_with(|| right.index.cmp(&left.index))
        })
}

fn parse_q_millis(raw_value: &str) -> Option<u16> {
    let value = raw_value.trim().parse::<f32>().ok()?;
    if !(0.0..=1.0).contains(&value) {
        return None;
    }

    Some((value * 1_000.0).round() as u16)
}
