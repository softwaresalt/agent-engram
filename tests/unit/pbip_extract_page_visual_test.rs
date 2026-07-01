//! Unit tests for PBIP page and visual entity extraction (062.007-T, plan
//! Unit 4).
//!
//! Verifies that the pbip extractor parses the project-definition report layout
//! into stable page-order, page-identity, visual-identity, visual-type, and
//! semantic-binding-hint entities — without touching semantic-model or TMDL
//! work.
//!
//! Tests: S-PPV-01..S-PPV-15

use engram::models::pbip::PbipBindingKind;
use engram::services::pbip_extract::{parse_page, parse_page_order, parse_visual};

/// Real-fixture `pages.json` mirroring
/// `tmp/...Report/definition/pages/pages.json`.
const PAGES_FIXTURE: &str = r#"{
  "$schema": "https://example/pages/1.4.0/schema.json",
  "pageOrder": [
    "VehicleRegistrationsLawEnforcement"
  ],
  "activePageName": "VehicleRegistrationsLawEnforcement"
}
"#;

/// Real-fixture `page.json` mirroring
/// `tmp/...Report/definition/pages/VehicleRegistrationsLawEnforcement/page.json`.
const PAGE_FIXTURE: &str = r#"{
  "$schema": "https://example/page/1.4.0/schema.json",
  "name": "VehicleRegistrationsLawEnforcement",
  "displayName": "Vehicle Registrations - Law Enforcement View",
  "displayOption": "FitToPage",
  "width": 1280,
  "height": 720,
  "visibility": "AlwaysVisible"
}
"#;

/// Real-fixture `visual.json` mirroring
/// `tmp/...visuals/cardTotalRegistrations/visual.json`.
const VISUAL_FIXTURE: &str = r#"{
  "$schema": "https://example/visualContainer/1.4.0/schema.json",
  "name": "cardTotalRegistrations",
  "position": { "x": 180, "y": 120, "z": 3, "width": 180, "height": 90, "tabOrder": 10 },
  "visual": {
    "visualType": "card",
    "query": {
      "queryState": {
        "Values": {
          "projections": [
            {
              "field": {
                "Measure": {
                  "Expression": { "SourceRef": { "Entity": "FactVehicleRegistrations" } },
                  "Property": "Total Registrations"
                }
              },
              "queryRef": "FactVehicleRegistrations.Total Registrations",
              "nativeQueryRef": "Total Registrations",
              "active": true
            }
          ]
        }
      }
    }
  }
}
"#;

// ── Page order (pages.json) ────────────────────────────────────────────────

/// S-PPV-01: `pages.json` yields the page order and active page.
#[test]
fn parse_page_order_returns_order_and_active() {
    let order = parse_page_order(PAGES_FIXTURE).expect("pages.json should parse");
    assert_eq!(order.order, vec!["VehicleRegistrationsLawEnforcement"]);
    assert_eq!(
        order.active_page,
        Some("VehicleRegistrationsLawEnforcement".to_string())
    );
}

/// S-PPV-02: `pages.json` without `activePageName` still yields order.
#[test]
fn parse_page_order_without_active_page() {
    let content = r#"{ "pageOrder": ["A", "B"] }"#;
    let order = parse_page_order(content).expect("parse order without active");
    assert_eq!(order.order, vec!["A", "B"]);
    assert_eq!(order.active_page, None);
}

/// S-PPV-03: `pages.json` without a `pageOrder` array is rejected.
#[test]
fn parse_page_order_rejects_without_page_order() {
    assert!(parse_page_order(r#"{ "activePageName": "A" }"#).is_none());
}

/// S-PPV-04: Non-JSON content is rejected.
#[test]
fn parse_page_order_rejects_non_json() {
    assert!(parse_page_order("not json").is_none());
}

// ── Page identity (page.json) ──────────────────────────────────────────────

/// S-PPV-05: `page.json` yields stable page identity (name + display name + ID).
#[test]
fn parse_page_returns_identity() {
    let path = "Report/definition/pages/VehicleRegistrationsLawEnforcement/page.json";
    let page = parse_page(PAGE_FIXTURE, path).expect("page.json should parse");
    assert_eq!(page.name, "VehicleRegistrationsLawEnforcement");
    assert_eq!(
        page.display_name,
        "Vehicle Registrations - Law Enforcement View"
    );
    assert_eq!(page.path, path);
    assert!(!page.id.is_empty(), "page must have a stable ID");
}

/// S-PPV-06: A `page.json` `display_name` falls back to `name` when absent.
#[test]
fn parse_page_display_name_falls_back_to_name() {
    let content = r#"{ "name": "OnlyName" }"#;
    let page = parse_page(content, "p/page.json").expect("parse name-only page");
    assert_eq!(page.name, "OnlyName");
    assert_eq!(page.display_name, "OnlyName");
}

/// S-PPV-07: The page ID is stable per path and distinct across paths.
#[test]
fn parse_page_id_is_stable_and_path_scoped() {
    let a = parse_page(PAGE_FIXTURE, "a/page.json").expect("a");
    let a_again = parse_page(PAGE_FIXTURE, "a/page.json").expect("a again");
    let b = parse_page(PAGE_FIXTURE, "b/page.json").expect("b");
    assert_eq!(a.id, a_again.id);
    assert_ne!(a.id, b.id);
}

/// S-PPV-08: A `page.json` without a `name` is rejected.
#[test]
fn parse_page_rejects_without_name() {
    assert!(parse_page(r#"{ "displayName": "x" }"#, "p/page.json").is_none());
}

/// S-PPV-09: Non-JSON content is rejected.
#[test]
fn parse_page_rejects_non_json() {
    assert!(parse_page("not json", "p/page.json").is_none());
}

// ── Visual identity, type, and bindings (visual.json) ──────────────────────

/// S-PPV-10: `visual.json` yields stable visual identity and type.
#[test]
fn parse_visual_returns_identity_and_type() {
    let path = "Report/definition/pages/P/visuals/cardTotalRegistrations/visual.json";
    let visual = parse_visual(VISUAL_FIXTURE, path).expect("visual.json should parse");
    assert_eq!(visual.name, "cardTotalRegistrations");
    assert_eq!(visual.visual_type, "card");
    assert_eq!(visual.path, path);
    assert!(!visual.id.is_empty(), "visual must have a stable ID");
}

/// S-PPV-11: `visual.json` extracts a measure semantic-binding hint.
#[test]
fn parse_visual_extracts_measure_binding() {
    let visual = parse_visual(VISUAL_FIXTURE, "v/visual.json").expect("parse");
    assert_eq!(visual.bindings.len(), 1, "one measure binding expected");
    let binding = &visual.bindings[0];
    assert_eq!(binding.kind, PbipBindingKind::Measure);
    assert_eq!(binding.entity, "FactVehicleRegistrations");
    assert_eq!(binding.property, "Total Registrations");
}

/// S-PPV-12: `visual.json` extracts a column semantic-binding hint.
#[test]
fn parse_visual_extracts_column_binding() {
    let content = r#"{
      "name": "barByCounty",
      "visual": {
        "visualType": "barChart",
        "query": {
          "queryState": {
            "Category": {
              "projections": [
                {
                  "field": {
                    "Column": {
                      "Expression": { "SourceRef": { "Entity": "DimCounty" } },
                      "Property": "CountyName"
                    }
                  }
                }
              ]
            }
          }
        }
      }
    }"#;
    let visual = parse_visual(content, "v/visual.json").expect("parse");
    assert_eq!(visual.bindings.len(), 1);
    let binding = &visual.bindings[0];
    assert_eq!(binding.kind, PbipBindingKind::Column);
    assert_eq!(binding.entity, "DimCounty");
    assert_eq!(binding.property, "CountyName");
}

/// S-PPV-13: A visual with no query bindings still parses, with empty bindings.
#[test]
fn parse_visual_without_bindings_is_ok() {
    let content = r#"{ "name": "decorativeShape", "visual": { "visualType": "shape" } }"#;
    let visual = parse_visual(content, "v/visual.json").expect("parse");
    assert_eq!(visual.visual_type, "shape");
    assert!(
        visual.bindings.is_empty(),
        "no query state should yield no bindings"
    );
}

/// S-PPV-14: A `visual.json` without a `visual` object is rejected.
#[test]
fn parse_visual_rejects_without_visual_object() {
    assert!(parse_visual(r#"{ "name": "x" }"#, "v/visual.json").is_none());
}

/// S-PPV-15: Non-JSON content is rejected.
#[test]
fn parse_visual_rejects_non_json() {
    assert!(parse_visual("not json", "v/visual.json").is_none());
}
