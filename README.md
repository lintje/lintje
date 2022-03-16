# Lintje

<div align="center">
  <b><a href="https://lintje.dev">Lintje.dev website</a></b>
</div>

---

Lintje is an opinionated linter for Git. It lints commit messages based on a
preconfigured set of [rules][rules] focussed on promoting communication between
people. The idea is to write commits meant for other people reading them during
reviews and debug sessions 2+ months from now.

- __No configuration__. Don't spend time configuring your Git commit linter and
  instead adopt a preconfigured set of [rules][rules].
- __Portable__. Lintje is a Rust project built for several
  [Operating Systems](#supported-operating-systems) and has no dependencies.
  Drop it into your project and it works.

## Documentation

Visit [Lintje.dev][website] for more information about Lintje, and the
[Lintje documentation](https://lintje.dev/docs/).

- [Installation](https://lintje.dev/docs/installation/)
- [Usage][usage]
    - [Git hook](https://lintje.dev/docs/git-hooks/)
    - [Git alias](https://lintje.dev/docs/git-alias/)
- [Rules documentation][rules]
- [Configuration](https://lintje.dev/docs/configuration/)
- [Getting help](https://lintje.dev/docs/support/)
- [Development documentation](#development)

## Example

Given the last commit in a project is this:

```
Fix bug
```

When running `lintje` to lint the last commit, the output will be:

```
$ lintje
Error[SubjectCliche]: The subject does not explain the change in much detail
  9a2ae29:1:1: Fix bug
    |
  1 | Fix bug
    | ^^^^^^^ Describe the change in more detail

Error[MessagePresence]: No message body was found
  9a2ae29:3:1: Fix bug
    |
  1 | Fix bug
  2 |
  3 |
    | ^ Add a message body with context about the change and why it was made

Error[BranchNameTicketNumber]: A ticket number was detected in the branch name
  Branch:1: fix-123
  |
  | fix-123
  | ^^^^^^^ Remove the ticket number from the branch name or expand the branch name with more details

1 commit and branch inspected, 3 errors detected
```

For more usage examples, see the [usage docs].

## Getting help

Need help with Lintje? Found a bug or have a question?

Reach out to me through the [issue tracker][issues],
[discussions][discussions], on Twitter
[@tombruijn](https://twitter.com/tombruijn) (DMs are open) or on any Slack team
you can find me on.

## Development

### Setup

Make sure [Rust](https://www.rust-lang.org/) is installed before continuing.

```
cargo build
```

### Testing

```
cargo test
```

### Building

[Docker](https://www.docker.com/) is required to build all the different target
releases using [cross](https://github.com/rust-embedded/cross).

To build all different targets, run the build script:

```
rake build
```

The build output can be found in the `dist/` directory.

### Releases

Before release all the supported targets will be build. See
[Building](#building) for more information about the build step.

To release all different targets, run the release script:

```
rake release
```

The release will be pushed to GitHub.

Finally update the
[Lintje Homebrew tap](https://github.com/tombruijn/homebrew-lintje).

## Code of Conduct

This project has a [Code of Conduct](CODE_OF_CONDUCT.md) and contributors are
expected to adhere to it.

[website]: https://lintje.dev
[rules]: https://lintje.dev/docs/rules/
[usage]: https://lintje.dev/docs/usage/
[issues]: https://github.com/tombruijn/lintje/issues
[discussions]: https://github.com/tombruijn/lintje/discussions
[installation]: https://lintje.dev/docs/installation/
