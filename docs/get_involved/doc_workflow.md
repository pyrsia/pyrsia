# Documentation Contribution Workflow

To contribute to the documentation (including blogs), you can follow the following guidelines for making documentation contributions.

All documents relevant to the project are written in the Markdown format. You can see the documentation
for the GitHub Flavor [here](https://github.github.com/gfm/) or you can use this
[quick guide](https://docs.github.com/en/get-started/writing-on-github/getting-started-with-writing-and-formatting-on-github/basic-writing-and-formatting-syntax).

All the documentation is visible on GitHub and <https://pyrsia.io> website, so it's important to keep in mind
there are some limitations and extra features that might be available. You might want to refer to the
[website's standard features](https://docusaurus.io/docs/markdown-features#standard-features) if you are unsure.

## Structure Guidelines

All the documentation for Pyrsia should live in the [pyrsia/pyrsia](https://github.com/pyrsia/pyrsia) repository
in the [`docs`](https://github.com/pyrsia/pyrsia/blob/main/docs) folder. Blogs belong in the same repository but should be
placed in the [`blog`](https://github.com/pyrsia/pyrsia/blob/main/blog) folder.

### `docs`

All sub-folders should have a `readme.md` with a good title, as a level one header, and the
[front matter for position](https://docusaurus.io/docs/api/plugins/@docusaurus/plugin-content-docs#sidebar_position)
the folder on the website. Make sure to pay attention to the other folder numbers.

All documents should have a `.md` file extension. If you need more customization you can use `.mdx`, see
[here](https://docusaurus.io/docs/markdown-features/react) for more information. All files need a level one heading to provide the
page with a good title.

### `blog`

All blogs should be placed in the root of the `blog` folder. Each one should be named `yyyy-mm-dd-<slug>.md`, where slug should be a unique
name for the blog (e.g. a short hand of the title); this is described [here](https://docusaurus.io/docs/api/plugins/@docusaurus/plugin-content-blog).

Some front matter is required at the top of the Markdown:

- title
- authors: \[name, title, image_url]
- tags

See [#720](https://github.com/pyrsia/pyrsia/pull/720) for an example.

In addition the [`draft` front matter](https://docusaurus.io/docs/api/plugins/@docusaurus/plugin-content-blog#draft) may be added
so the blog can be reviewed by the community before being made public on the website.

Blogs should not have use a level one heading, after the fron matter the opening paragraph or abstract should be next.
Section heading should begin with a level two heading.

All media assests should be external links since they will not be copied to the website when deployed from this repository.

## Previewing Changes

Currently it is only possible to preview your changes locally, you should be following the
[contributing guidelines](https://pyrsia.io/docs/get_involved/contributing/#dev-flow) and have forked the repository.
To preview the changes:

1. Fork the [website's repository](https://github.com/pyrsia/pyrsia.github.io) and clone your fork.
2. Modify [this line](https://github.com/pyrsia/pyrsia.github.io/blob/main/package.json#L6)
   - Change `pyrsia/pyrsia` to your fork, (e.g `octocat/pyrsia`)
   - Optionally, you can change the branch by replacing `main` with `your-branch-name`
   - You can [check this example](https://github.com/pyrsia/pyrsia.github.io/pull/66/commits/c317f9dab8f6bcde5f8588ca75858db72241930d)
3. Follow instructions described [here](https://github.com/pyrsia/pyrsia.github.io#website) for "local development"

If you make changes to your fork of the `pyrsia/pyrsia` repository, you can restart the local server to update the documentation.
You can make changes locally and the local serve will automatically update; do not forget to change them to your fork afterwards.
