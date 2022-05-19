# Documentation Contribution Workflow

To contribute to the documentation, you can follow the following guidlines to for making documentation contributions.

All documents relevant to the project are written in the Markdown format. You can see the documentation
for the GitHub Flavor [here](https://github.github.com/gfm/) or you can use this 
[quick guide](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax).

All the documentation is visible on GitHub and https://pyrsia.io website, so it's important to keep in mind
there are some limitations and extra features that might be availble. You might want to refer to the [website's standard features](https://docusaurus.io/docs/markdown-features#standard-features) if you are unsure.

## Structure Guidelines

All the documentation for Pyrsia should live in the [pyrsia/pyrsia](https://github.com/pyrsia/pyrsia) in the [`docs`](https://github.com/pyrsia/pyrsia/blob/main/docs) folder.

All subfolders should have a `readme.md` with a good title, as a level one header, and the
[front matter for position](https://docusaurus.io/docs/api/plugins/@docusaurus/plugin-content-docs#sidebar_position)
the folder on the website. Make sure to pay attention to the other folder numbers.

All documents should have a `.md` file extention. If you need more customization you can use `.mdx`, see 
[here](https://docusaurus.io/docs/markdown-features/react) for more imformation. All files need a level one heading to provide the
page with a good title.

## Previewing Changes

Currently it is only possible to preview you changes locally, you should be following the [contributing guidelines](https://pyrsia.io/docs/get_involved/contributing/#dev-flow) and have worked the repository. To preview the changes:

1. Clone the [website's repository](https://github.com/pyrsia/pyrsia.github.io) or your own fork.
2. Modify [this line](https://github.com/pyrsia/pyrsia.github.io/blob/main/package.json#L6)
   - Change `pyrsia/pyrsia` to your fork, (e.g `octocat/pyrsia`)
   - Optionally, you can change the branch by replacing `main` with `your-branch-name`
   - You can [check this example](https://github.com/pyrsia/pyrsia.github.io/pull/66/commits/c317f9dab8f6bcde5f8588ca75858db72241930d)
4. Follow instructions described [here](https://github.com/pyrsia/pyrsia.github.io#website) for "local development"

If you make changes to you fork of the `pyrsia/pyrsia` repository, you can restart the local server to update documentation.
You can make changes locally and the local serve will automatically update; do not forget to change them to your fork afterwards.
