# Bootstrap Governance

The initial bootstrap committee will consist of 5 individuals who are core stakeholders and/or contributors.

Members are (in alphabetical order by last name):

* Stephen Chin [@steveonjava](https://github.com/steveonjava), JFrog
* Chris Crone [@chris-crone](https://github.com/chris-crone), Docker
* Sudhindra Rao [@betarelease](https://github.com/betarelease), JFrog
* Steve Taylor [@sbtaylor15](https://github.com/sbtaylor15), DeployHub/Ortelius OS
* Johan Vos [@johanvos](https://github.com/johanvos), Lodgon

The committee MUST:

* Represent a cross-section of interests, not just one company
* Balance technical, architectural, and governance expertise since its initial mission is the establishment of structure around contributions, community, and decision-making
* Hold staggered terms, sufficient to ensure an orderly transition of power via elections as designed and implemented by the committee (see below for specific deliverables)
* Provide designated alternates in cases where quorum is required but not attainable with the current set of members
* Communicate with the Continuous Delivery Foundation on a regular cadence

## Committee Deliverables

The committee will be responsible for a series of specific artifacts and activities as outlined below.

### Initial Charter

This document will define how the committee is to manage the project until it has transitioned to an elected steering body, as well as what governance must be in place.
The Kubernetes Steering Committee Charter Draft serves as a good example.

A charter should cover all of the following topics:

* Scope of rights and responsibilities explicitly held by the committee
* Committee structure that meets the requirements above
* Election process, including:
  * special elections in the case someone resigns or is impeached
  * who is eligible to nominate candidates and how
  * who is eligible to run as a candidate
  * Voter registration and requirements
  * election mechanics such as
    * committee company representation quotas
    * Limits on electioneering
    * Responses to election fraud
  * How are changes to the charter enacted, and by what process
  * How are meetings conducted
    * Recorded or not, and if not, how is the information shared
    * How is work tracked? Example steering project board
    * Is there a member note taker, or is there a neutral facilitator role that exists outside of the committee?
    * Frequency, duration, and required consistency
  * Committee decision-making process, and specifically those areas of action that require more/less consensus, e.g. modifications the charter
  * Sub-Steering Committee governance structure (see this example)

## Transition Process

The transition process MUST:

* Organize, execute, and validate an election for replacing bootstrap members (they may re-run, but would need to be re-elected in order to stay)
* Define the term lengths for newly-elected individuals, ideally so not all members change out at once
* Provide documentation for the community and committee members sufficient to smoothly continue the established practices of the committee

## Contribution Process

The committee MUST define a contribution process that:

* Explains to potential contributors how/if they can add code to the repository/repositories
* Documents Workflow and management of pull requests
* Identifies who is authorized to commit or revert code
* Identifies automation is required for normal operations
* Defines how release decisions are made
  * Who is authorized to release and when.
  * Frequency limits
* Defines the documentation process
* Defines what Contributor License Agreement (CLA) process is required and how it is enforced through automation before code is merged

### Security/Vulnerability Reporting and Response Process

* Identify and document where vulnerability reporting can be done to the project
* Identify and document who is responsible for receiving vulnerability reports
* Document process responsible parties go through to triage and determine veracity of vulnerability
* Document process for facilitating fix, generating release update, and communicating vulnerability and fix to public

## Code of Conduct

The code of conduct MUST set expectations for contributors on expected behavior, as well as explaining the consequences of violating the terms of the code.
The [Contributor Covenant](https://www.contributor-covenant.org) has become the de facto standard for this language.

Members of the governance committee will be responsible for handling [Pyrsia's code of conduct](https://github.com/pyrsia/.github/blob/main/code-of-conduct.md)
violations via [conduct@cd.foundation](mailto:conduct@cd.foundation).

## Project Communication Channels

What are the primary communications channels the project will adopt and manage?
This can include Slack, mailing lists, an organized Stack Overflow topic, or exist only in GitHub issues and pull requests.

* Mailing list: [groups.google.com/g/pyrsia](https://groups.google.com/g/pyrsia).
* Slack: [#pyrsia](https://cdeliveryfdn.slack.com/join/shared_invite/zt-1eryue9cw-9YpgrfIfsTcDS~hGHchURg)

Governance decisions, votes and questions should take place on the pyrsia@googlegroups.com mailing list.

## Permissions and access

Members of the governing board will be given access to these resources:

* Google Groups Administrators
* [GitHub Org](https://github.com/orgs/pyrsia/teams/admins) Administrators
