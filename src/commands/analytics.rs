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

fn build_query(zone_id: &str, start: &NaiveDate, end: &NaiveDate) -> GraphQLQuery {
    let query = format!(
        r#"query {{
            viewer {{
                zones(filter: {{ zoneTag: "{}" }}) {{
                    httpRequests1dGroups(
                        filter: {{ date_geq: "{}", date_leq: "{}" }}
                        limit: 100
                        orderBy: [date_ASC]
                    ) {{
                        dimensions {{ date }}
                        sum {{
                            requests
                            responseStatusMap {{ edgeResponseStatus requests }}
                        }}
                    }}
                }}
            }}
        }}"#,
        zone_id, start, end
    );

    GraphQLQuery {
        query,
        variables: serde_json::json!({}),
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

fn print_status_table(groups: &[HttpRequestGroup]) {
    let warn = |n: u64| if n > 0 { " !" } else { "" };

    println!(
        "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Date", "Total", "2xx", "3xx", "4xx", "5xx"
    );
    println!("{}", "-".repeat(62));

    let totals = groups.iter().fold(
        StatusCounts {
            total: 0,
            s2xx: 0,
            s3xx: 0,
            s4xx: 0,
            s5xx: 0,
        },
        |mut acc, group| {
            let c = count_by_class(&group.sum);
            println!(
                "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8}{}",
                group.dimensions.date,
                c.total,
                c.s2xx,
                c.s3xx,
                c.s4xx,
                c.s5xx,
                warn(c.s5xx)
            );
            acc.total += c.total;
            acc.s2xx += c.s2xx;
            acc.s3xx += c.s3xx;
            acc.s4xx += c.s4xx;
            acc.s5xx += c.s5xx;
            acc
        },
    );

    if groups.len() > 1 {
        println!("{}", "-".repeat(62));
        println!(
            "{:<14} {:>8} {:>8} {:>8} {:>8} {:>8}{}",
            "Total",
            totals.total,
            totals.s2xx,
            totals.s3xx,
            totals.s4xx,
            totals.s5xx,
            warn(totals.s5xx)
        );
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
