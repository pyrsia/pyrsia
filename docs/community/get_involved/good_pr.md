# Good Pull Requests

Pull Requests are how changes are shared with the community. When authors add features or fix bug for code contributions it's
important to have the mind set:

> _I am asking others for a favor to review and give feedback on my work with the goal of delivering the best quality work_

With that in mind, what should authors do to make this process as smooth as possible for the reviewers?

## What makes a good Pull Request

* Link to an issue that has clear description so reviewers know what to expect.
* Keep the changes small, limit the scope of the PR to make it clear and concise.
* Fill in the PR template to give the most information as possible.
  * Clear title
  * Detailed description
  * How to test/verify/review the changes locally
* Screenshot of outcome if possible, visually it is easier to understand what happens.
* Add logs in to highlight what to expect when running the code locally.

## Signs of a weak Pull Request

* Large number of changed files.
* Lots of inline code documentation.
* Numerous questions which don't understand "why this was changed".

### Possible solutions

* Open new issues for the extra work you spot in the code if it takes more then 30 minutes.
* Focus on the issue at hand!
* Clearly call out changes in the description of code comments to inform the reviewer.
* Document design decision in the `docs/` folder.
* Share the link to a Google Docs describing the decision and choices made.
* Include any meeting records where the issue was discussed

## Test cases

It's always recommended to write tests for any code changes. Tests should describe both expected and undesirable scenarios.

Make sure the Pull Request has:

* Code that is readable by itself along with test cases that supplement the narrative of how the code works.

## Process of how you build PRs

It is worth reading our general [contributing guidelines](https://github.com/pyrsia/.github/blob/main/contributing.md#dev-flow).

Beyond that, any optional check(s) that fail should be brought up at the next team meeting so we can evaluate the significance.

## Review cycle

Pull Requests take time. It may take several passes for feedback and questions to be completely resolved.
Make sure to help others learn about the work you've done and appreciate the dedication to improving your work.

* Enough time to understand the code and any context behind it.
* Ability to demonstrate the function of the new code.
* We should have guidance about what reviewers should expect.
* Assigned reviewers questions should to be answered before merging

Follow the 30 minutes when evaluating suggestions and comments!
↪️ If the changes would take more then 30 minutes, it's probably out of scope so open a new issue to keep track of the improvements.
