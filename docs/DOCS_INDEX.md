# Janet School Docs Index

This folder mixes research doctrine, technical schema docs, and operator-facing guidance. If you are new to Janet School, start with the plain-language docs first.

## Start Here

1. [../README.md](../README.md)
2. [USER_MANUAL.md](USER_MANUAL.md)
3. [RUNBOOK.md](RUNBOOK.md)
4. [WINDOWS_ACCEPTANCE.md](WINDOWS_ACCEPTANCE.md)
5. [CONFIG_REFERENCE.md](CONFIG_REFERENCE.md)
6. [ABOUT.md](ABOUT.md)
7. [../RELEASE_CHECKPOINT.md](../RELEASE_CHECKPOINT.md)

## Plain-Language Operator Docs

- [USER_MANUAL.md](USER_MANUAL.md)
  Comprehensive, zero-knowledge-assumption guide for running and using Janet School.
- [RUNBOOK.md](RUNBOOK.md)
  Shorter practical workflows for common operator tasks.
- [WINDOWS_ACCEPTANCE.md](WINDOWS_ACCEPTANCE.md)
  Pass/fail Windows acceptance run sheet for build, GUI, mock run, artifacts, and local-LLM readiness.
- [CONFIG_REFERENCE.md](CONFIG_REFERENCE.md)
  Plain-language explanation of every config file and the main settings operators are likely to touch.
- [ABOUT.md](ABOUT.md)
  Publisher, stewardship, GitHub, outreach, and license summary in one place.
- [../RELEASE_CHECKPOINT.md](../RELEASE_CHECKPOINT.md)
  Short release or handoff template that pairs with the Windows acceptance run sheet.

## Research Doctrine

- [ALIGNMENT.md](ALIGNMENT.md)
  What Janet School is for, what it is not for, and the hard research guardrails.
- [ARCHITECTURE.md](ARCHITECTURE.md)
  System shape, boundaries, and data flow.

## Technical Schemas

- [CURRICULUM_SCHEMA.md](CURRICULUM_SCHEMA.md)
  Curriculum structure, validation rules, and intended item annotations.
- [TELEMETRY_SCHEMA.md](TELEMETRY_SCHEMA.md)
  Session outputs, event families, and logging rules.
- [ANALYSIS_SCHEMA.md](ANALYSIS_SCHEMA.md)
  Analysis categories, confidence posture, and report shapes.

## GUI Planning

- [GUI_PLAN.md](GUI_PLAN.md)
  GUI intent, operator surface goals, and phase plan.

## Maintainer Guides

- [CONFIG_REFERENCE.md](CONFIG_REFERENCE.md)
  Config meanings, path settings, and the safest way to edit them.
- [SKILL_EXTENSION_GUIDE.md](SKILL_EXTENSION_GUIDE.md)
  How to add a new deterministic skill cleanly across engine logic, config, tests, and session outputs.

## Suggested Reading By Role

- New operator:
  `README.md`, `USER_MANUAL.md`, `RUNBOOK.md`, `WINDOWS_ACCEPTANCE.md`, `CONFIG_REFERENCE.md`
- Research reviewer:
  `ALIGNMENT.md`, `ANALYSIS_SCHEMA.md`, `TELEMETRY_SCHEMA.md`
- Builder or maintainer:
  `ARCHITECTURE.md`, `WINDOWS_ACCEPTANCE.md`, `CONFIG_REFERENCE.md`, `SKILL_EXTENSION_GUIDE.md`, schema docs, `GUI_PLAN.md`

## One Important Reminder

The user-facing docs are intentionally simpler than the schema docs. If the two ever feel different in tone, prefer the schema and architecture docs for exact data contracts, and prefer the user manual for step-by-step operation.
