---
title: "Match all Spark SIMPLE_COMMENT line endings"
description: "Preserve backslash-LF continuation while terminating Spark line comments on CR, LF, and CRLF."
doc_type: learning
problem_type: "Spark SQL comment boundary mismatch"
category: "best-practices"
component: "Spark lineage SQL normalization"
resolution_type: "code-and-test"
severity: "medium"
date: 2026-08-10
shipment: "112-S"
pr: 331
tags:
  - "spark"
  - "sql"
  - "lineage"
  - "parser"
  - "comments"
---

## Problem

A byte scanner that treats only LF as a `--` comment terminator handles CRLF
incidentally but swallows genuine SQL after a bare CR. Conversely, treating
every LF as a terminator breaks Spark's backslash-LF continuation.

## Resolution

Match Spark's `SIMPLE_COMMENT` boundary narrowly:

- consume `\\` followed immediately by LF as comment continuation;
- terminate on bare CR or LF;
- let CRLF terminate at CR; and
- leave the terminal line-ending bytes available to the outer scanner.

Keep nested block-comment depth handling independent. Use exact controls for
backslash-LF continuation, CRLF termination, bare-CR termination, nested block
comments, quoted regions, and genuine `INSERT` recall.

## Prevention

When implementing a grammar fragment with a byte scanner, verify every
accepted line-ending form against the upstream grammar rather than relying on
the host platform's newline convention. Add the boundary controls before
changing the scanner and preserve a failing RED result for each behavior
change.
