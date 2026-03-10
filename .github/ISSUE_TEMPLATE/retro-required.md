---
name: Retro required (for closure)
description: Checklist to ensure a retrospective is written when closing this item
title: "[RETRO] <paste issue title>"
labels: [retro]
body:
  - type: markdown
    attributes:
      value: |
        Use this template to create a retrospective entry under `docs/retrospectives/`.
  - type: input
    id: issue_link
    attributes:
      label: Link to the closed issue/milestone
      placeholder: https://github.com/adamdost-0/Telescope/issues/123
    validations:
      required: true
  - type: input
    id: file_path
    attributes:
      label: Retro file path in repo
      placeholder: docs/retrospectives/ISSUE-123-short-slug.md
    validations:
      required: true
  - type: textarea
    id: summary
    attributes:
      label: Summary
      description: Paste the retro content (or write it directly here, then copy into the file)
      placeholder: |
        What shipped...
    validations:
      required: true
