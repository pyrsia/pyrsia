## What makes a good Pull Request

* Small focused issues -> small PRs -> easy reviews -> better cycle time
* Link to an issue with a very clear description so it’s clear what the changed code is supposed to do
* Create smaller issues that address the PRs specifically
* PRs should summarize what the changes are and be linked to an issue so that the reviewer knows what to expect.
* A PR should be linked to an issue and explain why that issue is solved by the PR. The PR should contain a test that fails before and succeeds after, making it clear there was an issue before (matching the linked issue) and solved after.
* Good PRs are small. Long review times are the enemy of small PRs.
* PRs that are not small discourage people from reviewing them quickly
* Ensure that the PR is associated with and issue that addresses the small part of the system
* Good description
* Primary problem that was fixed
* If this fixes other things that were affecting the system during this PR that should be included in the PR
Keywords used/format in the PR description to link and auto close
Try to complete the sentence “This PR ..”
Eg : Closes #10 addresses the issue of conflicting ports when running multiple peers
Adds configuration to provide port number
Adds default port number
* Should be strongly linked to the issue it addressed without confusion
* How to verify/test this PR?
* Add comments
* Prefer creating/linking to an issue to describe the design choice instead of adding long comments in the code.


## Test cases - that describe the expected and unexpected scenarios
* Code that is readable by itself along with test cases that supplement the readability

* Can have screenshot of outcome if possible, visually it’s more easy to understand what the PR is about especially if the reviewer has no context of PR
* Add log files/screenshots in to the issues to describe the Before state


## Process of how you build PRs
* How do you work on a second PR when the first PR is still being reviewed?
Work on the same branch the first PR was built

* Pull Request Automation Process
* The PR should automatically be annotated with a list of review instructions:
CLA should be ok - enforced by github
Pre-submit checks can be done automatically (syntax, code,...) - enforced by cargo `command`
Not required checks? - ask for feedback during the PR review process
How many reviewers are required - 2 - randomly assigned - enforced by github
The link to how to checkout the PR should be shown (avoiding that people clone others fork etc) - shortcuts are available in github (post links from the archived channel here - Sudhindra)
* While doing this, the progress should be visually clear - automation should clearly show failures
* Can the not required checks be made visually clear - 

## Good PR Review cycle
* Enough time to understand the code and context
* Ability to demonstrate the function of the new code
* We should have guidelines about what is expected from reviewers.
* Assigned reviewers questions to be answered for the PRs to be merge ready
* Actionable review comments with enough descriptive steps for the PR to be changed

