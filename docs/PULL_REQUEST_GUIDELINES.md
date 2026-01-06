# Pull Request Guidelines

## Overview

PRs are the lifeline of the team. They allow us to ship value, determine maintenance cost, and impact daily quality of life. Well-maintained code sustains velocity.

## Why

A quality pull request process enables sustained velocity and consistent delivery.

Pull requests allow us to:

- **Hold and improve quality**: Catch bugs and architectural issues early
- **Build team expertise**: Share knowledge through clear descriptions and thoughtful reviews
- **Stay customer focused**: Keep PRs tight and decoupled for incremental, reversible changes
- **Encourage ownership**: Clear domain ownership motivates high quality and reduces incidents

## SLA

| Metric | Target | Rationale |
|--------|--------|-----------|
| PR Review | Within half a working day | If reviews take longer than 1 working day, something needs improvement |

## Success Metrics

| Metric | Why |
|--------|-----|
| Time to PR Review | Fast reviews power the flywheel: shared context → quality code → maintainable codebase → iteration |
| Time from PR Open to Production | Deployed code reaches customers |
| Incidents | Quality authoring and review catches errors early |
| QA Regression Bugs | Quality authoring and review catches errors early |

## Authoring Guidelines

### Keep PRs Tight

PRs should make either deep changes (few files, significant logic) or shallow changes (many files, simple refactors).

- **< 500 LOC** (guideline; auto-generated or boilerplate may exceed this)
- **< 10 files** (guideline; renames may touch more files)

### Write Clear Descriptions

Enable reviewers to understand the problem and verify the solution. Good descriptions also serve as documentation.

### Test Thoroughly

Any code change impacts flows. Include:
- Manual testing
- Unit tests
- QA regression tests where appropriate

### Choose Reviewers Carefully

Select codebase owners and domain experts. Reach out early to allow reviewers to schedule time.

### Budget Time for Reviews

Allow time for comments, suggestions, and improvements. Code is written once but read many times.

### Consider Live Reviews

Synchronous reviews can resolve alignment faster. Document decisions in the PR for posterity.

## Reviewing Guidelines

### Review Within Half a Day

Fast reviews generate a flywheel of velocity and knowledge-sharing.

### Review in Detail

No rubber stamping. Given good descriptions and self-review, PRs should be relatively easy to review thoroughly.

## Codebase Ownership

### Unit Tests

Owners decide on unit test thresholds reflecting appropriate effort and business risk.

### Conventions

Team should have consensus on conventions. Ideally automated or linted; otherwise documented.

## FAQ

**Can we make an exception for tight timelines?**

Yes, exceptions are always possible. For large PRs, hold a retro to identify what could be done differently.

**When should authors seek reviewers?**

As early as possible. Reviewers of design artifacts (PPS/TDD) should likely also review the PR.
