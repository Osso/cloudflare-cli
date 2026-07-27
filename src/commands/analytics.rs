use anyhow::{Result, bail};
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::find_zone_id;
use crate::client::Client;

#[derive(Debug, Serialize)]
struct GraphQLQuery {
    query: String,
    variables: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<GraphQLData>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphQLData {
    viewer: Viewer,
}

#[derive(Debug, Deserialize)]
struct Viewer {
    zones: Vec<ZoneData>,
}

#[derive(Debug, Deserialize)]
struct ZoneData {
    #[serde(rename = "httpRequests1dGroups")]
    http_requests: Vec<HttpRequestGroup>,
}

#[derive(Debug, Deserialize)]
struct HttpRequestGroup {
    dimensions: Dimensions,
    sum: Sum,
}

#[derive(Debug, Deserialize)]
struct Dimensions {
    date: String,
}

#[derive(Debug, Deserialize)]
struct Sum {
    requests: u64,
    #[serde(rename = "responseStatusMap")]
    response_status_map: Vec<ResponseStatus>,
}

#[derive(Debug, Deserialize)]
struct ResponseStatus {
    #[serde(rename = "edgeResponseStatus")]
    edge_response_status: u16,
    requests: u64,
}

const STATUS_CODES_QUERY: &str = r#"query StatusCodes($zoneTag: String, $start: String, $end: String) {
    viewer {
        zones(filter: { zoneTag: $zoneTag }) {
            httpRequests1dGroups(
                filter: { date_geq: $start, date_leq: $end }
                limit: 100
                orderBy: [date_ASC]
            ) {
                dimensions { date }
                sum {
                    requests
                    responseStatusMap { edgeResponseStatus requests }
                }
            }
        }
    }
}"#;

fn build_query(zone_id: &str, start: &NaiveDate, end: &NaiveDate) -> GraphQLQuery {
    GraphQLQuery {
        query: STATUS_CODES_QUERY.to_string(),
        variables: serde_json::json!({
            "zoneTag": zone_id,
            "start": start.to_string(),
            "end": end.to_string(),
        }),
    }
}

struct StatusCounts {
    total: u64,
    s2xx: u64,
    s3xx: u64,
    s4xx: u64,
    s5xx: u64,
}

fn count_by_class(sum: &Sum) -> StatusCounts {
    let mut counts = StatusCounts {
        total: sum.requests,
        s2xx: 0,
        s3xx: 0,
        s4xx: 0,
        s5xx: 0,
    };

    for s in &sum.response_status_map {
        match s.edge_response_status {
            200..=299 => counts.s2xx += s.requests,
            300..=399 => counts.s3xx += s.requests,
            400..=499 => counts.s4xx += s.requests,
            500..=599 => counts.s5xx += s.requests,
            _ => {}
        }
    }

    counts
}

fn warn_suffix(count: u64) -> &'static str {
    if count > 0 { " !" } else { "" }
}

fn print_table_header() {
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Date", "Total", "2xx", "3xx", "4xx", "5xx"
    );
    println!("{}", "-".repeat(62));
}

fn print_table_row(label: &str, counts: &StatusCounts) {
    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8}{}",
        label,
        counts.total,
        counts.s2xx,
        counts.s3xx,
        counts.s4xx,
        counts.s5xx,
        warn_suffix(counts.s5xx)
    );
}

fn print_group_rows(groups: &[HttpRequestGroup]) -> StatusCounts {
    groups.iter().fold(
        StatusCounts {
            total: 0,
            s2xx: 0,
            s3xx: 0,
            s4xx: 0,
            s5xx: 0,
        },
        |mut totals, group| {
            let counts = count_by_class(&group.sum);
            print_table_row(&group.dimensions.date, &counts);
            totals.total += counts.total;
            totals.s2xx += counts.s2xx;
            totals.s3xx += counts.s3xx;
            totals.s4xx += counts.s4xx;
            totals.s5xx += counts.s5xx;
            totals
        },
    )
}

fn print_status_table(groups: &[HttpRequestGroup]) {
    print_table_header();
    let totals = print_group_rows(groups);

    if groups.len() > 1 {
        println!("{}", "-".repeat(62));
        print_table_row("Total", &totals);
    }
}

pub async fn status_codes(client: &Client, zone: &str, days: u32) -> Result<()> {
    let zone_id = find_zone_id(client, zone).await?;

    let end = Utc::now().date_naive();
    let start = end - chrono::Duration::days(days as i64);

    let gql_request = build_query(&zone_id, &start, &end);
    let response: GraphQLResponse = client.graphql(&gql_request).await?;

    if let Some(errors) = response.errors {
        for err in &errors {
            eprintln!("GraphQL error: {}", err.message);
        }
        bail!("GraphQL query failed");
    }

    let data = response
        .data
        .ok_or_else(|| anyhow::anyhow!("No data returned"))?;

    if data.viewer.zones.is_empty() {
        bail!("No zone data found for '{}'", zone);
    }

    let groups = &data.viewer.zones[0].http_requests;

    if groups.is_empty() {
        println!("No HTTP request data for the last {} day(s)", days);
        return Ok(());
    }

    print_status_table(groups);

    Ok(())
}
