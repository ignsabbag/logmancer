---
name: create-issue
description: "Trigger: draft, create, or open a GitHub issue, bug report, feature request, or epic for Logmancer. Check duplicates and prepare English issues with consistent scope, acceptance criteria, and labels."
license: Apache-2.0
metadata:
  version: "1.0"
---

## Activation Contract

Use this skill to draft or create Logmancer GitHub issues. Discussion or local
drafting does not authorize publication. Do not implement the issue, create a
commit or PR, or edit existing issues unless separately requested.

## Hard Rules

- Write issue titles, headings, and body text in English, even when the conversation is in another language. Preserve quoted errors, logs, and code verbatim.
- Use the current repository's confirmed GitHub remote; do not infer the destination solely from the working directory name. Specify `--repo OWNER/REPO` in GitHub CLI commands.
- Search open and closed issues before publishing. Do not claim that duplicate checks passed if access failed or the search was incomplete.
- Separate reported observations, verified code facts, and investigation hypotheses. Never invent reproduction steps, environment details, test results, or root causes.
- Keep scope proportional to the request. Describe desired behavior and constraints, not an unnecessarily prescriptive implementation plan.
- Require verifiable acceptance criteria with unchecked `- [ ]` items. Do not mark unimplemented work complete.
- Use existing `type:*` and `area:*` labels, without duplicating them with legacy `bug`, `enhancement`, or `documentation` labels.
- Apply `status:approved` only when the user explicitly approves implementation. Approval to publish an issue is not approval to implement it.
- Do not create labels, milestones, dependencies, or child issues without authorization. Do not apply `size:exception`, which describes a PR review exception.
- Do not include repetitive pre-flight checklists, empty sections, session narration, AI attribution, or a routine statement that no code was changed.
- Redact credentials and sensitive paths or log contents in public issue text. Retain only the details needed to understand or reproduce the problem.

## Workflow

### 1. Establish the request and context

- Read the repository-root `AGENTS.md` and inspect the GitHub remote.
- Determine whether the user wants a draft or publication, and whether implementation approval was explicitly given.
- Identify the issue type, affected runtimes, intended outcome, and important boundaries.
- Inspect only relevant source files and documentation when technical context is useful. For cross-cutting architectural proposals, find and read the relevant records under `docs/adr/`; do not read every ADR or rewrite accepted decisions.
- Ask about missing details only when they materially affect the issue. Mark non-blocking unknowns explicitly instead of guessing.

### 2. Check existing issues and labels

Use the authenticated GitHub CLI or an equivalent available GitHub integration:

```bash
gh issue list --repo OWNER/REPO --state all --search "RELEVANT KEYWORDS" --limit 100 --json number,title,state,url
gh issue view NUMBER --repo OWNER/REPO --json number,title,body,labels,state,url
gh label list --repo OWNER/REPO --limit 100
```

Search with several relevant terms rather than only the proposed title. Inspect
plausible matches and broaden or paginate truncated results when needed.

- If an existing issue covers the same work, present its link and ask how to proceed instead of opening a duplicate.
- A closed issue may indicate a regression rather than a duplicate. Confirm the distinction and reference it in the new draft.
- Reference related work as `Related to #N`, `Follow-up to #N`, `Part of #N`, or `Depends on #N` only when the relationship is established.
- If GitHub access is unavailable, produce a draft and report which checks remain pending; do not publish.

### 3. Draft a proportional issue

Use a concise title in this form:

```text
TYPE(SCOPE): concise description
```

Use `fix`, `feat`, `refactor`, `chore`, `docs`, `test`, `ci`, `build`, or `epic`
as appropriate. Omit `(SCOPE)` when no useful single scope exists. Scopes such
as `core`, `web`, `desktop`, `tui`, `config`, `registry`, or `distribution` are
descriptive; they do not replace labels.

Choose one of the structures below. Remove all placeholders and omit optional
sections that add no useful information.

#### Simple change: feature, maintenance, refactor, or documentation

```markdown
## Problem
Describe the current limitation and its impact. Use `## Goal` instead when a goal is clearer.

## Proposed Solution
Describe the agreed direction and boundaries. Use `## Scope` with bullets for several deliverables.

## Acceptance Criteria
- [ ] Observable outcome.
- [ ] Relevant regression boundary or verification.
```

#### Bug

```markdown
## Problem
Describe the observed failure and its impact, with a minimal example if useful.

## Steps to Reproduce
1. Known reproduction step.
2. Action that exposes the failure.

## Expected Behavior
Describe the correct result.

## Actual Behavior
Describe the observed result and relevant error output.

## Acceptance Criteria
- [ ] The reported failure is resolved or isolated, according to the agreed scope.
- [ ] Relevant edge cases and regressions are covered.
```

For a bug without confirmed reproduction, replace invented steps with an explicit
note explaining what is known and what remains to be isolated. For an
investigation-only issue, acceptance criteria should describe evidence or
diagnostics to deliver, not promise a fix.

#### Epic

```markdown
## Goal
Describe the overall outcome.

## Scope
- Included capability.

## Phases
1. Reviewable stage of work.

## Acceptance Criteria
- [ ] End-to-end outcome.

## Child Issues
- [ ] #N Existing child issue and its purpose.

## Out of Scope
- Explicitly excluded capability.
```

Do not fabricate child issue numbers. If children do not exist, omit `Child
Issues` and propose the breakdown in `Phases`. Creating and linking children is
a separate authorized action.

Optional sections for any structure:

- `Technical Context`: verified code facts and clearly labeled hypotheses.
- `Environment`: reported OS, version, runtime, or build details relevant to a bug.
- `Relevant Files`: a short list of verified paths, not a repository inventory.
- `Dependencies` or `Parent`: established relationships to existing work.
- `Out of Scope`: exclusions that prevent likely scope creep.
- `Alternatives Considered`: meaningful tradeoffs actually discussed.
- `Design Record` or `References`: relevant ADRs, documentation, or sanitized evidence.

Acceptance criteria should describe observable behavior and applicable tests or
validation. Avoid vague phrases such as "feels smoother", "works where possible",
or an undecided "wraps or stops". Specify the agreed behavior or identify the
decision as an explicit investigation deliverable. Do not repeat the same long
requirements list in several sections. Mention changelog or quality gates only
when they are a specific deliverable, not as boilerplate in every issue.

### 4. Select labels and obtain authorization

Select exactly one primary type from the labels confirmed to exist:

| Work | Label |
|------|-------|
| Bug or regression | `type:bug` |
| New feature or enhancement | `type:feature` |
| Behavior-preserving refactor | `type:refactor` |
| Maintenance, tooling, CI, packaging, or test-only work | `type:chore` |
| Documentation-only work | `type:docs` |
| Cross-cutting tracking issue | `type:epic` |

Add only relevant existing area labels: `area:core`, `area:web`, `area:desktop`,
`area:tui`, and `area:ux`. Multiple areas are valid when the work actually
affects them; shared Web/Desktop UI changes may need both. Do not invent an area
label for CI or packaging when none exists. Add `status:approved` only under the
explicit implementation-approval rule above.

Present the title, body, proposed labels, and duplicate-check findings for
review. Clarify unresolved scope decisions before publication. A user explicitly
asking to publish an agreed draft authorizes creation without another redundant
confirmation; otherwise obtain approval of the proposed issue first.

### 5. Publish and verify

After authorization, publish the reviewed body through `gh issue create` with
an explicit repository and labels. Prefer `--body-file` with an approved
temporary file outside the repository to preserve Markdown, backslashes, and
code blocks without shell expansion. Do not add unrelated local files.

Read back the created issue with `gh issue view` and verify its title, body, and
labels. If creation returns an ambiguous failure, check whether the issue was
created before retrying to avoid duplicates. Report mismatches or failed
verification without claiming success or making unapproved follow-up edits.

## Output Contract

- Draft only: provide the English title, body, proposed labels, related or duplicate issues, and unresolved questions. State that it has not been published.
- Published: return the verified issue URL and labels, distinguishing publication from implementation approval.
- Blocked: explain the blocker and provide the usable draft when possible.
