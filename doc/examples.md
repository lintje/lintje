# Examples

This page contains examples of how to use Lintje in different scenarios and
setups. This page is a work in progress.

## Continuous integration

### Semaphore

[Semaphore](https://semaphoreci.com/) has an environment variables that contains
the range of commits that were pushed for a given workflow. This tests all the
commits that are included in a Pull Request or Git push on a branch.

Add the following command to your build as a separate job.

```
$ lintje $SEMAPHORE_GIT_COMMIT_RANGE
```

This command with the `$SEMAPHORE_GIT_COMMIT_RANGE` variable wil resolve to a
commit range like this:

```
$ lintje 5c84719708b9b649b9ef3b56af214f38cee6acde...92d87d5c0dd2dbb7a68ecb27df43d1b164fd3e30
```

Read more about
[$SEMAPHORE_GIT_COMMIT_RANGE](https://docs.semaphoreci.com/ci-cd-environment/environment-variables/#semaphore_git_commit_range)
in the Semaphore docs.
