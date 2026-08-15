use anyhow::Result;
use serde::Deserialize;

use super::ApiResponse;
use crate::client::Client;

#[derive(Debug, Deserialize)]
pub struct AbuseReportListResponse {
    pub reports: Vec<AbuseReportSummary>,
}

#[derive(Debug, Deserialize)]
pub struct AbuseReportSummary {
    pub id: String,
    pub cdate: String,
    pub domain: String,
    #[serde(rename = "type")]
    pub report_type: String,
    pub status: String,
    #[serde(default)]
    pub mitigation_summary: Option<MitigationSummary>,
}

#[derive(Debug, Deserialize)]
pub struct Submitter {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub company: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MitigationSummary {
    #[serde(default)]
    pub accepted_url_count: u32,
    #[serde(default)]
    pub pending_count: u32,
    #[serde(default)]
    pub active_count: u32,
    #[serde(default)]
    pub in_review_count: u32,
    #[serde(default)]
    pub cancelled_count: u32,
    #[serde(default)]
    pub removed_count: u32,
    #[serde(default)]
    pub external_host_notified: bool,
}

#[derive(Debug, Deserialize)]
pub struct AbuseReportDetail {
    pub id: String,
    pub cdate: String,
    pub domain: String,
    #[serde(rename = "type")]
    pub report_type: String,
    pub status: String,
    #[serde(default)]
    pub urls: Vec<String>,
    #[serde(default)]
    pub original_work: Option<String>,
    #[serde(default)]
    pub submitter: Option<Submitter>,
    #[serde(default)]
    pub mitigation_summary: Option<MitigationSummary>,
}

pub async fn list(client: &Client) -> Result<()> {
    let path = format!("/accounts/{}/abuse-reports", client.account_id());
    let response: ApiResponse<AbuseReportListResponse> = client.get(&path).await?;

    if response.result.reports.is_empty() {
        println!("No abuse reports found");
        return Ok(());
    }

    for report in response.result.reports {
        let urls = report
            .mitigation_summary
            .as_ref()
            .map(|m| m.accepted_url_count)
            .unwrap_or(0);
        println!(
            "● {} [{}] {} ({}) - {} URL(s)",
            report.id, report.report_type, report.domain, report.status, urls
        );
        println!("  Date: {}", report.cdate);
    }

    Ok(())
}

pub async fn show(client: &Client, report_id: &str, json: bool) -> Result<()> {
    let path = format!(
        "/accounts/{}/abuse-reports/{}",
        client.account_id(),
        report_id
    );

    if json {
        let raw = client.get_raw(&path).await?;
        println!("{}", raw);
        return Ok(());
    }

    let response: ApiResponse<AbuseReportDetail> = client.get(&path).await?;
    let r = response.result;

    println!("Report ID:     {}", r.id);
    println!("Type:          {}", r.report_type);
    println!("Domain:        {}", r.domain);
    println!("Status:        {}", r.status);
    println!("Date:          {}", r.cdate);

    if let Some(sub) = r.submitter {
        println!("\nSubmitter:");
        if let Some(name) = sub.name {
            println!("  Name:        {}", name);
        }
        if let Some(company) = sub.company {
            println!("  Company:     {}", company);
        }
        if let Some(email) = sub.email {
            println!("  Email:       {}", email);
        }
    }

    if let Some(work) = r.original_work {
        println!("\nOriginal Work:\n  {}", work);
    }

    if !r.urls.is_empty() {
        println!("\nReported URLs ({}):", r.urls.len());
        for url in r.urls {
            println!("  - {}", url);
        }
    }

    if let Some(m) = r.mitigation_summary {
        println!("\nMitigation Summary:");
        println!("  Accepted URLs:          {}", m.accepted_url_count);
        println!("  Pending:                {}", m.pending_count);
        println!("  In Review:              {}", m.in_review_count);
        println!("  Host Notified:          {}", m.external_host_notified);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_abuse_report_list() {
        let json = r#"{
            "result": {
                "reports": [
                    {
                        "id": "eebcab2542155a49",
                        "cdate": "2026-08-15T01:03:55.792199Z",
                        "domain": "mangahelpers.com",
                        "type": "DMCA",
                        "status": "accepted",
                        "mitigation_summary": {
                            "accepted_url_count": 1,
                            "pending_count": 0,
                            "active_count": 0,
                            "in_review_count": 0,
                            "cancelled_count": 0,
                            "removed_count": 0,
                            "external_host_notified": true
                        }
                    }
                ]
            },
            "success": true
        }"#;

        let res: ApiResponse<AbuseReportListResponse> = serde_json::from_str(json).unwrap();
        assert_eq!(res.result.reports.len(), 1);
        assert_eq!(res.result.reports[0].id, "eebcab2542155a49");
        assert_eq!(res.result.reports[0].report_type, "DMCA");
        assert_eq!(res.result.reports[0].domain, "mangahelpers.com");
    }

    #[test]
    fn deserialize_abuse_report_detail() {
        let json = r#"{
            "result": {
                "id": "eebcab2542155a49",
                "cdate": "2026-08-15T01:03:55.792199Z",
                "domain": "mangahelpers.com",
                "type": "DMCA",
                "status": "accepted",
                "urls": [
                    "https://mangahelpers.com/forum/threads/hunter-x-hunter-chapter-418-spoilers-discussion.3038665/"
                ],
                "original_work": "Viz Manga - Original content",
                "submitter": {
                    "company": "Viz Manga",
                    "email": "infringement@removeyourmedia.com",
                    "name": "Eric Green"
                },
                "mitigation_summary": {
                    "accepted_url_count": 1,
                    "pending_count": 0,
                    "active_count": 0,
                    "in_review_count": 0,
                    "cancelled_count": 0,
                    "removed_count": 0,
                    "external_host_notified": true
                }
            },
            "success": true
        }"#;

        let res: ApiResponse<AbuseReportDetail> = serde_json::from_str(json).unwrap();
        assert_eq!(res.result.id, "eebcab2542155a49");
        assert_eq!(res.result.urls.len(), 1);
        assert_eq!(
            res.result.submitter.as_ref().unwrap().name.as_deref(),
            Some("Eric Green")
        );
    }
}
