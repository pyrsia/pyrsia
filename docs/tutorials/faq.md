---
sidebar_position: 4
---

# FAQ

## Can I run a Pyrsia node?

Yes, have a look at [Quick Installation](/docs/tutorials/quick-installation.mdx)
and one of the package specific tutorials on [Docker](/docs/tutorials/docker/) or [Maven](/docs/tutorials/maven/).

## Do I have to participate in artifact distribution?

The more nodes participate in artifact distribution, the better of course. But if
you only want to run a Pyrsia node to consume artifacts, that works as well.

## How can I clean a Pyrsia storage on my local machine?

To reset your environment, you can remove the `pyrsia` directory where all data created through the Pyrsia transactions such as cached artifacts and transparency logs are stored.

This sometimes solves issues which occur due to backward-incompatible changes.

On Linux, you can run this command:

```shell
rm -rf /usr/local/var/pyrsia
```

On Windows and macOS, the location varies up to the way you install Pyrsia.
If you just followed [Quick Installation](/docs/tutorials/quick-installation.mdx), use the following commands:

```shell
# Windows (Command Prompt)
rd /s c:\Pyrsia\Pyrsia\service\pyrsia
```

```shell
# macOS
rm -rf $(brew --prefix)/var/pyrsia
```
