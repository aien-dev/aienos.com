use maud::{Markup, PreEscaped};

pub fn render_progress_page() -> Markup {
    let raw = include_str!("../../../../public/data/project-progress.json");
    let data: serde_json::Value = serde_json::from_str(raw).expect("Valid project progress dataset");
    let summary = &data["summary"];
    let mut body = include_str!("../../templates/progress.html").to_string();
    for (marker, key) in [("__LINES__", "loc"), ("__REVISIONS__", "commits"), ("__CI__", "ci")] {
        let value = summary[key].as_u64().expect("Progress metric must be a nonnegative integer");
        body = body.replace(marker, &value.to_string());
    }
    // GitHub titles are untrusted strings; escaping '<' prevents closing the JSON script element.
    let safe_data = raw.replace('<', "\\u003c");
    body = body.replace("__DATA__", &format!("<script type=\"application/json\" id=\"ag-data\">{safe_data}</script>"));
    PreEscaped(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_is_a_real_page_with_the_same_published_measurements() {
        let page = render_progress_page().into_string();
        assert!(page.contains("<h1>Project progress</h1>"));assert!(page.contains("id=\"ag-data\""));assert!(page.contains("id=\"ag-code\""));assert!(page.contains("id=\"ag-activity\""));
        assert!(!page.contains("__LINES__"));assert!(!page.contains("__DATA__"));assert!(!page.contains('\u{2014}'));assert!(!page.contains('\u{2013}'));
    }
}
