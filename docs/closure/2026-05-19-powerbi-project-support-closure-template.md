---
title: "Power BI project support — Closure template"
type: closure-template
date: 2026-05-19
feature: 061-F
status: draft
owner: softwaresalt
---

## Purpose

Use this template when closing a shipment that changes the Power BI indexing
surface. Capture the shipped scope, the exact runtime verification that passed,
and the rollback signals operators should watch immediately after merge.

## Required Shipment Metadata

Fill in:

* shipment ID and title
* PR number
* merge commit SHA
* working branch
* tasks completed in the shipment

## Runtime Verification Checklist

Run and record:

1. `engram sync`
2. `engram search "Total Sales" --content-type powerbi --format text`
3. `engram query-graph --format text`

Healthy results should show:

* the `powerbi` source is accepted and indexed
* Power BI search results include report, page, visual, table, or measure hits
* graph traversal reaches semantic-model, table, measure, or relationship nodes

## Supported Surface to Mention

Document which of these shipped in the release unit:

* `report.json`
* `model.bim`
* `definition/**/*.tmdl`

Also restate the current TMDL limitation:

* structural extraction is supported
* full DAX lineage is not yet derived

## Known-Gap Recording

If the shipment includes TMDL work, note any remaining parity gaps between
JSON-backed semantic models and TMDL-based semantic model folders.

## Monitoring Plan

Manual observation is sufficient unless a later shipment changes runtime scale
or introduces rollout risk.

Minimum observation window:

* the merge itself
* the first post-merge `engram sync` on a representative Power BI workspace
* one follow-up operator verification pass for search and graph results

## Rollback Trigger

Rollback if either of these appears after merge:

* `engram sync` skips, fails, or regresses on the configured `powerbi` source
* Power BI search or graph results lose expected TMDL semantic-model coverage

## Rollback Procedure

Use `git revert --no-edit -m 1 <merge_commit>` if runtime behavior regresses
on `main`.
If the rollback is closure-only, revert the closure artifact commit and restore
the backlog state that existed before shipment archival.

## Closure Notes

Every Power BI closure should record:

* the exact registry source path used for verification
* one representative search example
* one representative graph traversal example
* whether TMDL coverage changed
* the owner responsible for the manual observation window
