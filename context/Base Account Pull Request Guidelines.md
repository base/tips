# Base Account Pull Request Guidelines

Status **Living**  
Updated Jun 16, 2025  
Created Oct 20, 2024

[Overview](#overview)

[Why](#why)

[SLA](#sla)

[Success Metrics](#success-metrics)

[Guidelines](#guidelines)

[Authoring](#authoring)

[Reviewing](#reviewing)

[Owning a Codebase](#owning-a-codebase)

[Appendix](#appendix)

[FAQ](#faq)

[Resources](#resources)

[Notes](#notes)

# Overview {#overview}

*“Honesty in small things is not a small thing.”*

- *Robert C. Martin, Clean Code: A Handbook of Agile Software Craftsmanship*

PRs are the lifeline of the team. It’s what allows us to ship value, decides our future in terms of maintenance cost, and has a high impact on our daily QOL. When code is well-maintained and untangled, velocity is sustained.

This document lays out SLAs, guidelines, and how we think about pull requests as a world-class engineering team.

## Why {#why}

Having a quality pull request process allows us to build with sustained velocity and deliver improvements and features to our users.

Pull requests allow us to:

* **Hold and improve the bar on quality:** we can catch bugs and architectural code smells early, before these have reached QA or users  
* **Build a championship team and mentor top talent**: by knowledge-sharing through clear descriptions and deep, thoughtful reviews, this will help our team become stronger  
* **Stay customer focused with repeatable innovation:** by keeping the PRs tight and decoupled, this will allow us to consistently ship (or roll back) incremental improvements to our customers  
* **Encourage ownership:** by having clear ownership around domains, this will motivate authors and reviewers to hold the quality high, allowing for the codebase to be malleable to fit the business needs while reducing incidents and bugs

As engineers, pull requests are a key part of our day-to-day life, whether it’s authoring or reviewing them. By holding a high bar, it will also improve our daily quality of life.

## SLA {#sla}

| Name | SLA | Why |
| :---- | :---- | :---- |
| **PR Review** | Reviewed within half a working day | Pull requests are encouraged to be reviewed within half a working day. If they take longer than 1 working day, likely, something needs to be improved. |

## Success Metrics {#success-metrics}

| Metric | Why |
| :---- | :---- |
| **Time to PR Review** | Getting a review quickly and swiftly is one of the fastest ways to get unblocked. By having fast reviews, it powers the flywheel of shared context → quality code → maintainable codebase → iteration. |
| **Time from PR Open to Production** | Getting code merged is one thing. Getting it deployed is how it gets in front of customers. |
| **\# of Incidents** | By having a quality author and review process, errors should be caught during the pull request process. |
| **\# of QA regression bugs** | By having a quality author and review process, errors should be caught during the pull request process. |

# Guidelines {#guidelines}

## Authoring {#authoring}

* **PRs should be tight.** This allows for teammates to review deeply and thoroughly. PRs should either make deep changes (creating two new functions) or a shallow change across a breadth of files (renaming a function).

  * PRs should be \<500 LOC. This is a guideline. There may be PRs which may be higher LOC (eg: if it’s auto-generated, boilerplate, or scaffolding). There also may be PRs which are 1-2 lines.

  * PRs should touch \<10 files. This is a guideline. If the PR is focused on renaming across a codebase, it could be 30+ files, but with minimal or no other business logic change.

* **PRs should be well-described.** This allows for teammates to understand the problem and what the PR sets out to do. Importantly, it also allows for verification of the code and is well-documented for posterity.

* **PRs should be well-tested.** Any change in code will impact flows. These flows should be well-tested. These can be manually tested and unit tested. Additionally, a QA regression test could be added.

* **Consider who the reviewers are.** Reviewers are ideally owners of the codebase and/or those with deep knowledge of the domain. By reaching out and finding dedicated reviewers early in the process, it also gives a heads up to the reviewers, allowing them to schedule and prioritize reviews.

* **Budget time for reviews.** Allow the reviewers time to comment and suggest. Even more importantly, allow there to be time to make improvements. Code is written once, but read much more times.

* **Consider hosting a synchronous, live review.** Sometimes, it’s easier to communicate live with the reviewers to align (or disagree and commit). Please work the alignment back into the PR for posterity.

* **Examples:**

  * [https://github.cbhq.net/wallet/wallet-mobile/pull/27738](https://github.cbhq.net/wallet/wallet-mobile/pull/27738)  
  * [https://github.cbhq.net/wallet/wallet-mobile/pull/26092](https://github.cbhq.net/wallet/wallet-mobile/pull/26092)

## Reviewing {#reviewing}

* **PRs should be reviewed within half a day.** This is one of the things worth prioritizing for teammates as it generates a flywheel to improve velocity and knowledge-sharing. At the same time, PRs should be relatively easy to review, given the description, self-review, and code quality.

* **PRs should be reviewed in detail.** No rubber stamping. 

## Owning a Codebase {#owning-a-codebase}

**Unit Test**  
[Base Wallet Unit Test + Lint SLAs](https://docs.google.com/document/u/0/d/1Ai3UDVDR3Hq1P-smfaeuclxDMQYSJjwlAPFyERraJCI/edit)

**Unit Test Thresholds**  
The owners should decide on unit test thresholds. These thresholds should reflect the appropriate LOE and risk for the business given what code is responsible for.

**Conventions**  
The team should have consensus on conventions, or lack of. Ideally, conventions are automated or linted. Else, they should be documented.

# Appendix {#appendix}

## FAQ {#faq}

**There is a tight timeline and it’s easier to get everything into 1 humongous PR. Can we make an exception?**  
Short answer: yes, we can always make an exception. There is no hard and fast rule. If there are humongous PRs, I’d recommend having a retro on it to see what could have been done differently.

**When should we, as authors, seek out reviewers?**  
Short answer: as early as possible. This can differ based on the type of work.

If there is an artifact (PPS/TDD), the reviewers of the artifact likely should also be pull request reviewers. Transitively, reviewers of the pull requests should be reviewers of the artifact.

## Resources {#resources}

* [\[Code Red\] Bar Raiser Code Review Program](https://docs.google.com/document/d/1bzYI2gdnNZI9MqvHyTrD7aZRUfrakVedLRe2gw8b114/edit?tab=t.0#heading=h.ismr2neqrl10)  
* [Wallet Pull Request Guild (PRG) Readout](https://docs.google.com/document/u/0/d/1nyE26o9DwQnTstJBrwMYgr6qdQPe71YrluzyiMOU89A/edit)

## Notes {#notes}

Oct 24, 2024

Questions

* What are our actual problems with our current code and review process?  
  * Breaking ERC specs  
    * Break spec on eth request accounts for Pano  
      * Documentation may have solved this  
    * Additional parameter on connection  
  * Time to review is longer than ideal  
    * Observation: authors repost requesting reviews in Slack  
    * Hypotheses  
      * Lack of context  
      * Lack of ownership  
* Does having people not labeled as “Bar Raiser” set the wrong culture?  
  * Do we want a culture where some people can absolve themselves of the responsibility to raise the bar?  
* Regardless of bar-raisers, we could take it down one level. What do we care about?  
* [Wallet Pull Request Guild (PRG) Readout](https://docs.google.com/document/u/0/d/1nyE26o9DwQnTstJBrwMYgr6qdQPe71YrluzyiMOU89A/edit)  
* Less in-flight projects  
* Implement code owners

\> Bar Raisers for a given component or system are a subset of the broader owning team, typically consisting of more experienced engineers. For repositories or subdirectories with Bar Raisers, PRs must be either authored by a Bar Raiser or receive approval from one before merging. All existing review and merge rules still apply in addition to the Bar Raiser requirement. 

Domains

* Transaction / Signing: [Cody Crozier](mailto:cody.crozier@coinbase.com), [Lukas Rosario](mailto:lukas.rosario@coinbase.com), [Arjun Dureja](mailto:arjun.dureja@coinbase.com)  
* Sessions: [Spencer Stock](mailto:spencer.stock@coinbase.com), [Jake Feldman](mailto:jake.feldman@coinbase.com), [Felix Zhang](mailto:felix.zhang@coinbase.com)  
* SDK: [Felix Zhang](mailto:felix.zhang@coinbase.com), [Jake Feldman](mailto:jake.feldman@coinbase.com), [Spencer Stock](mailto:spencer.stock@coinbase.com), [Conner Swenberg](mailto:conner.swenberg@coinbase.com)  
* BE: [Sam Luo](mailto:sam.luo@coinbase.com), [Adam Hodges](mailto:adam.hodges@coinbase.com)  
* Smart contracts: [Amie Corso](mailto:amie.corso@coinbase.com), [Conner Swenberg](mailto:conner.swenberg@coinbase.com)  
* Infra: ?

Proposal

* Opt-in ✅  
* Everyone should maintain code and hold a high bar (be a bar raiser) ✅  
* Further discussion  
  * What it means to be a PR reviewer  
  * What improvements we can make  
    * Faster PR reviews  
    * Higher quality  
* Breaking ERC specs (retro this)  
* Other ways we can raise the bar