# Pyrsia RFCs

Many changes, including bug fixes and documentation improvements can be implemented and reviewed via the
normal GitHub pull request workflow.

Some changes though are "substantial", and we ask that these be put through a bit of a design process and produce
a consensus among the Pyrsia community. See [when to follow this process](#when-you-need-to-follow-this-process) below.

## Summary

The process can be broken down as follows:

1. [Reach out on community forms and open an issue](#before-creating-an-rfc)
2. [Follow the template and open a PR](#the-proposal-process)
3. [Attend our meeting and promote your proposal](#approach-the-community)
4. [Schedule a review meeting](#reviewing-a-proposal)

Lastly, [Proposal resolution](#proposal-resolution) may take several different forms.

## Why are RFCs needed

An RFCs is a design document providing information to the Pyrsia community. The RFC should provide a concise technical specification of
the feature and a rationale for the feature. Strong RFCs included detailed research on the idea and may be accompanied by a proof of concept

We intend RFCs to be the primary mechanisms for proposing major new features, for collecting community input on an issue, and for documenting
the design decisions that have gone into Pyrsia.

The RFC author is strongly encouraged to facilitate the discussion when the proposal is brought up at community events.
Because the RFCs are maintained as text files in a versioned repository, their revision history is the historical record of the feature proposal.

## When you need to follow this process

You should consider using this process if you intend to make "substantial" changes to Pyrsia or its documentation. Some examples that would benefit
from an RFC are:

- A new feature that creates new API surface area.
- The removal of features that already shipped as part of the release channel.
- The introduction of new idiomatic usage or conventions.

The RFC process is a great opportunity to get more eyeballs on your proposal before it becomes a part of a Pyrsia.
Quite often, even proposals that seem "obvious" can be significantly improved once a wider group of interested people have a chance to weigh in.

The RFC process can also be helpful to encourage discussions about a proposed feature as it is being designed, and incorporate important
constraints into the design while it's easier to change, before the design has been fully implemented.

## Before Creating an RFC

A hastily-proposed RFC can hurt its chances of acceptance. Low quality proposals, proposals for previously-rejected features, or those that don't fit
into the near-term roadmap, may be quickly rejected, which can be demotivating for the unprepared contributor. Laying some groundwork ahead of the RFC
can make the process smoother.

Although there is no single way to prepare for submitting an RFC, it is generally a good idea to pursue feedback from other project developers beforehand,
to ascertain that the RFC may be desirable; having a consistent impact on the project requires concerted effort toward consensus-building.

The most common preparations for writing and submitting an RFC include talking the idea over on our official Slack channel, discussing the topic in a
GitHub issue.

As a rule of thumb, receiving encouraging feedback from long-standing project developers, and particularly members of the relevant sub-team is a good indication
that the RFC is worth pursuing.

## The Proposal Process

In short, to get a major feature added, one must first get the RFC merged into the RFC repository as a markdown file. At that point the RFC is "active" and may
be implemented.

- Fork this repository [pyrsia/pyrsia](https://github.com/pyrsia/pyrsia/fork)
- Copy `docs/rfc/0000-template.md` to `docs/rfc/0000-my-feature.md`
  - where 'my-feature' is the title in kebab case; don't assign a number yet.
- Please put in enough time and research in putting a proposal together and we strongly encourage getting some feedback from part of the team before making your proposal final.
- Submit a pull request. As a pull request, the proposal will receive feedback from the larger community, and the author should be prepared to revise it in response.

### Approach the Community

Build consensus and integrate feedback. Proposals that have broad support are much more likely to make progress than those that don't receive any comments.

### Reviewing a Proposal

Periodically, the team will attempt to review the active proposals. We try to discuss proposals at the bi-weekly team
["Architecture"](https://pyrsia.io/docs/get_involved/#attend-a-community-meeting) meeting, we schedule additional meetings as need. Actions are recorded in the meeting minutes.

### Proposal resolution

Eventually, the team will decide whether the proposal is a candidate for adoption.

- A proposal can be modified based upon feedback from the team and community. Significant modifications may trigger a new final comment period.
- A proposal may be rejected by the team after public discussion has settled and comments have been made summarizing the rationale for rejection. A member of the team should then close the associated pull request.
- A proposal may be accepted. A team member will merge the proposal's associated pull request, at which point the proposal will become adopted.

## After acceptance

Once a proposal is accepted, then authors may implement it. This may mean submitting a pull request to the repository or putting some other process into place.
Acceptance however does not mean that resources are committed to the work; instead it means that the group is open to the change taking place.

Modifications to accepted proposals can be done in followup PRs.

## Implementing a proposal

The author of a proposal is not obligated to implement it. Of course, the proposal author (like any other community member) is welcome to post an implementation for review.

***

## Inspirations

This has been derived from other community driven projects. But in the end the changes to this process can be proposed to the team and this process is open to being updated.

- [React Native Proposals](https://github.com/react-native-community/discussions-and-proposals)
- [Rust RFC](https://github.com/rust-lang/rfcs)
- [Python PEPS](https://www.python.org/dev/peps)
