\# Codex Build Plan: Janet School Standalone Rust/WebView Research Rig



\## Mission



Build Janet School as a standalone Rust application with a WebView GUI.



Janet School is not an education app, not an AI training platform, and not a curriculum product. It is a cognitive mapping / abstraction-hunting research apparatus.



The system places a deterministic MCM student inside a controlled curriculum scaffold, uses an LLM teacher to generate and run curriculum sessions, captures exhaustive telemetry, and analyzes whether reusable structure, transfer behavior, boundary pressure, anomaly clusters, and abstraction-candidate mechanisms appear over successive runs.



The first build must stand alone. Do not make it ChattyCog-dependent yet. ChattyCog compatibility can be added later through export/import surfaces, session summaries, manifests, and module adapters.



\---



\## Hard Guardrails



1\. Do not turn Janet School into an ed-tech product.

2\. Do not make the MCM answer with an LLM.

3\. Do not hide LLM reasoning inside Janet’s core.

4\. Do not collapse anomalies into normal scoring.

5\. Do not claim “abstraction discovered” from one success.

6\. Do not create a “positive lane” that treats abstraction as a known achieved thing.

7\. Do not treat starter curriculum tags as the ontology of cognition.

8\. Do not hardcode a toy five-question demo and call it curriculum.

9\. Do not integrate ChattyCog in the first implementation.

10\. Do not optimize UI polish over telemetry, schema, reproducibility, and analysis correctness.



Use provisional language everywhere:



\* “candidate”

\* “possible”

\* “flagged”

\* “requires review”

\* “unknown structure”

\* “boundary pressure”

\* “category mismatch”

\* “emergent structure candidate”



Never use:



\* “proven abstraction”

\* “positive abstraction lane”

\* “confirmed intelligence”

\* “known cognitive law”

\* “education success”



\---



\## Core Roles



\### 1. LLM Teacher



The LLM teacher is allowed to:



\* generate curriculum modules

\* generate concept ladders

\* generate examples

\* generate questions

\* provide feedback

\* suggest remediation

\* suggest transfer probes

\* provide advisory evaluation notes after a run



The LLM teacher is not allowed to:



\* answer for Janet

\* edit Janet’s telemetry

\* hide uncertainty

\* declare abstractions proven

\* rewrite anomalies into neat outcomes

\* bypass the deterministic MCM core



Implement the teacher behind a trait/interface:



\* `TeacherBackend`



&#x20; \* `generate\_curriculum(request) -> CurriculumDraft`

&#x20; \* `generate\_question(context) -> CurriculumItem`

&#x20; \* `evaluate\_answer(context, janet\_answer, telemetry) -> TeacherEvaluation`

&#x20; \* `suggest\_next\_step(session\_state) -> TeacherAdvice`

&#x20; \* `summarize\_session(session\_data) -> TeacherSessionNotes`



Provide two backends:



\* `MockTeacherBackend` for deterministic tests

\* `LocalLlmTeacherBackend` for local llama.cpp / OpenAI-compatible localhost calls



No cloud API in v0.1.



\---



\### 2. MCM Student



The MCM is deterministic.



It may use:



\* explicit memory

\* approved deterministic skills

\* skill manifest

\* human approval ledger

\* deterministic matching

\* deterministic refusal policy

\* deterministic reasoning traces



It may not use:



\* hidden LLM calls

\* teacher answer leakage

\* semantic shortcuts from the LLM

\* invisible chain-of-thought

\* unlogged state mutation



The MCM answer pipeline should be inspectable:



1\. receive question

2\. normalize input

3\. classify surface features

4\. query explicit memory

5\. identify candidate skills

6\. check skill approval ledger

7\. either execute approved deterministic skill, answer from memory, compose partial attempt, or refuse

8\. emit answer

9\. emit full telemetry trace



\---



\## Curriculum System



The curriculum is scaffold, not theory.



The first standalone version must support full curriculum generation, not a small toy test.



Minimum curriculum generation requirements:



\* multiple domains

\* multiple concepts per domain

\* multiple teaching items per concept

\* multiple probe items per concept

\* near-transfer probes

\* far-transfer probes

\* cross-representation probes

\* composition probes

\* boundary probes

\* open-structure probes



Suggested initial scaffold domains:



1\. attention and discrimination

2\. matching and sameness/difference

3\. sorting and categorization

4\. sequencing and order

5\. quantity and comparison

6\. basic numeracy

7\. language comprehension

8\. relation words

9\. spatial reasoning

10\. temporal reasoning

11\. cause and effect

12\. functional problem solving

13\. social/pragmatic scenario reasoning

14\. rule following and exception handling

15\. pattern recognition

16\. abstraction and transfer probes



A “full run” should be able to generate hundreds of items, not five.



Suggested minimum for v0.1 full curriculum generation:



\* at least 12 domains

\* at least 5 concepts per domain

\* at least 5 teaching items per concept

\* at least 3 probe items per concept

\* at least 1 boundary or open-structure probe per concept



That gives a minimum target of roughly 480+ items.



Smoke tests can use a tiny curriculum fixture, but the actual application must support full curriculum generation and session execution.



\---



\## Curriculum Item Schema



Every curriculum item should allow hypothesis tags without enforcing them as truth.



Suggested fields:



\* `item\_id`

\* `domain\_id`

\* `concept\_id`

\* `item\_type`

\* `prompt`

\* `expected\_answer`

\* `acceptable\_answers`

\* `teaching\_context`

\* `difficulty`

\* `surface\_domain`

\* `intended\_relations`

\* `expected\_skills`

\* `novelty\_class`

\* `probe\_role`

\* `boundary\_kind`

\* `representation\_type`

\* `composition\_parts`

\* `notes`

\* `created\_by`

\* `created\_at`

\* `curriculum\_version`

\* `schema\_version`



Important rule:



Curriculum annotations describe what the experimenter or teacher thinks the item is testing. They do not decide what Janet actually did.



\---



\## Telemetry Requirements



Capture all telemetry.



Every interaction should log:



\* session id

\* run id

\* timestamp

\* curriculum version

\* item id

\* domain id

\* concept id

\* teacher backend id

\* teacher prompt

\* generated question

\* intended structure

\* expected answer

\* Janet answer

\* correctness judgment

\* teacher feedback

\* MCM memory reads

\* MCM memory writes

\* candidate skills

\* approved skills

\* blocked skills

\* executed skill

\* refusal reason

\* reasoning steps

\* confidence / uncertainty state

\* policy checks

\* latency

\* token counts for teacher calls

\* model config

\* random seed where applicable

\* observed structure

\* structure fit

\* anomaly flags

\* raw event trace hash

\* code version hash if available



Required output files per session:



\* `session\_config.json`

\* `curriculum\_generated.jsonl`

\* `curriculum\_validated.jsonl`

\* `teacher\_calls.jsonl`

\* `interactions.jsonl`

\* `mcm\_trace.jsonl`

\* `telemetry.jsonl`

\* `memory\_events.jsonl`

\* `skill\_events.jsonl`

\* `refusal\_events.jsonl`

\* `transfer\_probes.jsonl`

\* `anomaly\_events.jsonl`

\* `analysis\_report.json`

\* `analysis\_report.md`

\* `session\_summary.json`



Use append-only JSONL for event streams.



\---



\## Structure Fit and Anomaly Logic



Each interaction should classify fit as one of:



\* `matched`

\* `partial\_match`

\* `mismatch`

\* `unknown`



Each interaction may include anomaly flags such as:



\* `unexpected\_success`

\* `unexpected\_failure`

\* `unexpected\_refusal`

\* `unexpected\_skill\_path`

\* `unlabeled\_transfer`

\* `structure\_mismatch`

\* `boundary\_pressure`

\* `category\_mismatch`

\* `possible\_emergent\_structure`

\* `repeated\_near\_miss`

\* `teacher\_taxonomy\_insufficient`



Do not classify first-seen success as abstraction.



Instead, classify analysis events into:



\### Known Pattern Events



Examples:



\* exact recall

\* approved skill on intended item

\* expected transfer inside tagged category



\### Boundary Pressure Events



Examples:



\* approval-blocked matches

\* adjacent-skill near misses

\* refusals close to known structures

\* partial compositional attempts

\* inconsistent refusal behavior across similar items



\### Emergent Structure Candidate Events



Examples:



\* repeated success where intended tags fit poorly

\* stable success across items with different intended relations

\* repeated skill path divergence with correct answers

\* cross-item clustering not predicted by the current taxonomy

\* recurring mismatch patterns that suggest the current labels are too coarse



\---



\## Analyzer



Build a rule-based analyzer first.



The analyzer should produce:



\* confirmed signals

\* boundary signals

\* emergent signals

\* unknown structure candidates

\* repeated anomaly clusters

\* category mismatch clusters

\* cross-session pattern summary

\* caution notes

\* recommended next probes



The analyzer must not overclaim.



Every emergent result should include:



\* supporting event ids

\* why it was flagged

\* what current tags failed to explain

\* whether the pattern repeated

\* what probe would test it next

\* confidence level

\* “requires human review” flag



\---



\## Rust Project Shape



Suggested structure:



```text

janet-school-rs/

&#x20; Cargo.toml

&#x20; src/

&#x20;   main.rs

&#x20;   app.rs

&#x20;   config/

&#x20;   curriculum/

&#x20;   teacher/

&#x20;   mcm/

&#x20;   skills/

&#x20;   memory/

&#x20;   telemetry/

&#x20;   analysis/

&#x20;   session/

&#x20;   gui/

&#x20;   storage/

&#x20;   util/

&#x20; web/

&#x20;   index.html

&#x20;   app.js

&#x20;   styles.css

&#x20; config/

&#x20;   app\_config.json

&#x20;   teacher\_config.json

&#x20;   mcm\_config.json

&#x20;   skill\_manifest.json

&#x20;   skill\_approvals.json

&#x20; data/

&#x20;   sessions/

&#x20;   aggregated/

&#x20; docs/

&#x20;   ALIGNMENT.md

&#x20;   ARCHITECTURE.md

&#x20;   TELEMETRY\_SCHEMA.md

&#x20;   CURRICULUM\_SCHEMA.md

&#x20;   ANALYSIS\_SCHEMA.md

&#x20;   GUI\_PLAN.md

```



Recommended Rust crates:



\* `serde`

\* `serde\_json`

\* `anyhow`

\* `thiserror`

\* `chrono`

\* `uuid`

\* `tracing`

\* `tracing-subscriber`

\* `reqwest`

\* `tokio`

\* `schemars`

\* `jsonschema`

\* `clap`



For WebView:



Prefer a simple Rust backend plus static web UI.



Acceptable options:



\* `wry` + `tao`

\* or Tauri if it remains simple and does not swallow the project



The GUI must not become the core. The session runner and analysis engine should work headlessly.



\---



\## GUI Requirements



The WebView GUI should expose:



1\. Setup Page



&#x20;  \* teacher backend

&#x20;  \* model path / localhost endpoint

&#x20;  \* curriculum generation size

&#x20;  \* session name

&#x20;  \* output folder

&#x20;  \* run mode: smoke / full / analysis-only



2\. Curriculum Page



&#x20;  \* generated curriculum tree

&#x20;  \* domain list

&#x20;  \* concept list

&#x20;  \* item count

&#x20;  \* probe count

&#x20;  \* warning if too small



3\. Run Page



&#x20;  \* start / pause / stop

&#x20;  \* current domain

&#x20;  \* current concept

&#x20;  \* current item

&#x20;  \* teacher question

&#x20;  \* Janet answer

&#x20;  \* teacher feedback



4\. Live Telemetry Page



&#x20;  \* memory reads/writes

&#x20;  \* skill candidates

&#x20;  \* approved/blocked skills

&#x20;  \* refusal events

&#x20;  \* structure fit

&#x20;  \* anomaly flags



5\. Analysis Page



&#x20;  \* confirmed signals

&#x20;  \* boundary signals

&#x20;  \* emergent candidate signals

&#x20;  \* unknown structure candidates

&#x20;  \* repeated anomalies

&#x20;  \* recommended next probes



6\. Export Page



&#x20;  \* open session folder

&#x20;  \* export Markdown report

&#x20;  \* export JSON bundle

&#x20;  \* copy ChattyCog handoff summary later



\---



\## Build Phases



\### Phase 0: Alignment Before Code



Before writing application code, create:



\* `docs/ALIGNMENT.md`

\* `docs/ARCHITECTURE.md`

\* `docs/CURRICULUM\_SCHEMA.md`

\* `docs/TELEMETRY\_SCHEMA.md`

\* `docs/ANALYSIS\_SCHEMA.md`

\* `docs/GUI\_PLAN.md`



These docs must explicitly preserve:



\* abstraction-hunting purpose

\* scaffold-not-theory rule

\* deterministic MCM integrity

\* all telemetry

\* anomaly preservation

\* confirmed/boundary/emergent separation

\* no positive-lane claims

\* standalone-first build

\* ChattyCog compatibility later



Stop after Phase 0 for human review.



\### Phase 1: Skeleton



Create Rust workspace/app skeleton.



Implement:



\* config loading

\* session folder creation

\* append-only JSONL writer

\* schema structs

\* basic CLI commands

\* basic WebView shell



No LLM calls yet.



\### Phase 2: Deterministic MCM



Implement:



\* explicit memory store

\* skill manifest

\* skill approval ledger

\* deterministic candidate skill matcher

\* deterministic skill executor

\* refusal policy

\* MCM trace output



Add tests proving the MCM does not call the LLM.



\### Phase 3: Teacher Backend



Implement:



\* mock teacher backend

\* local LLM teacher backend

\* teacher call logging

\* curriculum generation

\* question generation

\* feedback generation

\* advisory evaluation



All teacher calls must be logged.



\### Phase 4: Session Runner



Implement full session loop:



1\. generate or load curriculum

2\. validate curriculum

3\. run teaching items

4\. run probes

5\. prevent probe feedback from contaminating memory

6\. log all events

7\. produce session summary



\### Phase 5: Analyzer



Implement rule-based analyzer:



\* known pattern events

\* boundary pressure events

\* emergent structure candidate events

\* repeated anomalies

\* mismatch clusters

\* open-structure probe results

\* Markdown and JSON reports



\### Phase 6: GUI Integration



Connect GUI to:



\* config

\* curriculum generation

\* session execution

\* live telemetry stream

\* report viewer

\* session exports



Keep GUI thin.



\### Phase 7: Acceptance Tests



Add tests for:



\* curriculum schema validation

\* full curriculum generation minimum item counts

\* telemetry writes

\* MCM deterministic-only answering

\* skill approval blocking

\* refusal logging

\* teacher calls logged separately

\* probe feedback not written to memory

\* anomaly flagging

\* boundary pressure classification

\* emergent candidate classification without overclaiming

\* session report sections



\---



\## v0.1 Acceptance Criteria



v0.1 is acceptable only when:



1\. It builds on Windows.

2\. It runs as a standalone Rust app.

3\. The WebView GUI opens.

4\. A smoke session can run with mock teacher.

5\. A local LLM teacher backend can generate a larger curriculum.

6\. The curriculum is not a five-question toy.

7\. The deterministic MCM answers without LLM calls.

8\. Every interaction emits telemetry.

9\. Teacher calls are logged separately.

10\. Skill governance is visible.

11\. Refusals are logged as meaningful events.

12\. Transfer probes are separated from teaching.

13\. Probe feedback does not contaminate memory.

14\. Analyzer produces confirmed, boundary, and emergent sections.

15\. Emergent results are called candidates, not proof.

16\. Session output folder contains JSONL logs, JSON summary, and Markdown report.

17\. ChattyCog integration is not required for v0.1.



\---



\## First Codex Task



Do not start by coding the whole app.



First, read the existing Janet School doctrine and docs, then produce the Phase 0 documentation set only.



After Phase 0, stop and wait for human review.



