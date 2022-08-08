# Contributing code to Pyrsia

Before contributing to Pyrsia, you should know a few things. From completing a contributor license agreement to understanding the different levels of participation, the below information will help you get started.

## Sign the CLA

You will need to complete a Contributor License Agreement (CLA) before your pull request can be accepted. This agreement testifies that you are granting us permission to use the source code you are submitting, and that this work is being submitted under appropriate license that we can use it.

For each pull request(code, documentation content, blogs or graphics), all commit authors are required to sign the [Contributing License Agreement](https://jfrog.com/cla/). We are using [CLA Assistant](https://cla-assistant.io/pyrsia/pyrsia.github.io) which requires commit email to match your GitHub account. You can view signed CLAs through their site.

If your organization would like to become a contributing organization, please have the appropriate individual complete the [Contributor License Agreement](https://cla-assistant.io/pyrsia/pyrsia.github.io). We welcome organizations of any size to be part of solving the supply chain gaps and encourage you to get your organization involved.

## Read the Code of Conduct

Please make sure to read and observe the [Code of Conduct and Community Values](../code-of-conduct.md).

## Understand PRs

All submissions, from code to content, will require a review. Where possible, GitHub pull requests will be used for this purpose. Consult [GitHub Help](https://help.github.com/articles/about-pull-requests/) for more information on using pull requests. For more details, check out our [Contributing Guidelines](./get_involved/contributing/).

## Development Workflow

Pyrsia follows the ["Forking Workflow"](https://blog.devgenius.io/git-forking-workflow-bbba0226d39c). You can see GitHub's
[About collaborative development models](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/getting-started/about-collaborative-development-models#fork-and-pull-model) for more details.

You can follow the instructions in [Development Environment Setup](local_dev_setup.md) to setup your environment to match how the team compiles and builds code.

To contribute follow the next steps:

1. Comment in the corresponding issue that you want to contribute. If there is no open issue, we strongly suggest
   opening one to gather feedback from the team.
2. Fork the [Pyrsia repository](https://github.com/pyrsia/pyrsia/fork) and [create a branch](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/creating-and-deleting-branches-within-your-repository#creating-a-branch)
   from the `main` branch and develop your fix and/or feature as discussed in the previous step. See
   [About forks](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/working-with-forks/about-forks) for help.
3. Try to keep your branch updated with the `main` branch to avoid conflicts. See
   [Syncing a fork](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/working-with-forks/syncing-a-fork).
4. Please make sure to [link any related issue](https://docs.github.com/en/issues/tracking-your-work-with-issues/linking-a-pull-request-to-an-issue)
   to the PR, referring to the issue of step 1.
5. Please follow the [submit pr](./submit_pr.md) process to verify your changes before submitting a PR.

When opening your PR, `CODEOWNERS` should fire and assign your request to one or more `pyrsia` organization teams.
They will then review and help with merging accepted changes.

### Pull Requests

PRs are a great way to share what you are working on and get early feedback. Make sure to [open as a draft](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/about-pull-requests#draft-pull-requests) if you are looking for early feedback.
Before opening a pull request it's recommended to "clean your commit history" and only keep the commits that address the issue. This makes it easier to review by breaking down the work and removing some of the clutter and noise of regular development. Check out [steps to clean git history](https://medium.com/@catalinaturlea/clean-git-history-a-step-by-step-guide-eefc0ad8696d) and [keeping git commit history clean](https://about.gitlab.com/blog/2018/06/07/keeping-git-commit-history-clean/) to learn more.

When PRs are "ready for review", there's a few house keeping 🧹 items to keep in mind:

- Follow the recommendations on the [Good PR](good_pr.md) document to ensure your PR will be acceptable to the team
- Make sure to give your PRs a **great title**. These will be the commit messages and should be treated as such.
- Do _not_ worry about squashing, that is done automatically by GitHub.
  - It's ideal to clean up any commit messages before confirming the merge to reduce the noise.
- Try to avoid force pushing your branch. GitHub forces reviewers to restart since it loses their progress.
- When synchronizing your branch, prefer using merge. Check out [Syncing a fork](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/working-with-forks/syncing-a-fork) for more details and guidance.

### Approval Process

Request reviews from the [`@pyrsia/collaborators`](https://github.com/orgs/pyrsia/teams/collaborators) team to assign team members for the PR.
They are responsible for making sure the PR is reviewed in a timely manner; they are expected to make time. Approvals are **not** limited to the assigned reviewers, anyone on the team can and should review each PR.

Specific individuals or "topic teams" may also be assigned (only after collaborators has been assigned so the GitHub automation can work properly). Approvals from "topic teams" are highly sought after but pull requests are _strongly encouraged_ to include reviews from the team at large.

All pull requests require:

- 2 approvals (from any team member)
- All required checks passing

If there are optional checks that fail, it's best to ask the reviewers and bring up the failure at the next team meeting.

### Project Board

[Learn more](https://docs.github.com/en/issues/organizing-your-work-with-project-boards/managing-project-boards/about-project-boards)

All our work is being tracked on our [Project Board](https://github.com/orgs/pyrsia/projects/3).

### Labels

[Learn how](https://docs.github.com/en/issues/using-labels-and-milestones-to-track-work/managing-labels#applying-a-label)

Labels are used to sort and describe issues and pull requests. Some labels are usually reserved for one or the other, though most labels may be applied to both.

They may be used to:

- highlight the state of completion, such as "Triage" or "Blocked"
- organizing according to the source code relevant to issues or the source code changed by pull requests, such as "Blockchain", "Discovery", or "Network"

Let us know if you have any feedback on these instructions or have suggestions to improve these.
