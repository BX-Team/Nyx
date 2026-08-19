<!--
^ Summarise what this changes and why, above this comment. ^

Title this pull request as a Conventional Commit — `feat(scope): …`, `fix: …`.
Release notes are generated from commit titles, so a title that does not parse
lands under "💬 Other" instead of its section.

If it closes an issue, write "Closes #123" so GitHub links the two.
If it changes behaviour a user can see, say what the old behaviour was.
-->

## Things done

<!-- Check what applies. These are not hard requirements — they tell a reviewer
     what you have already covered and where to look. Delete a group that has
     nothing to do with this change rather than leaving it unchecked. -->

- Tested, as applicable:
  - [ ] Ran the affected screen and walked through the behaviour this changes.
  - [ ] Covered by a new or updated test.
  - [ ] Nothing to test — documentation, formatting or build-only change.
- [ ] New user-facing strings go through `t!` and the key exists in **all three**
      locale files (`locales/en-US.yml`, `ru-RU.yml`, `zh-CN.yml`). A key present
      in only one falls back silently.
- [ ] No colour literal outside `ui/theme.rs` — new colours are tokens.
- Platform reach, where this touches the tray, the system proxy, the privileged
  service or window handling:
  - [ ] Checked on Linux.
  - [ ] Checked on Windows.
  - [ ] Both `cfg` branches updated, or the change is genuinely platform-neutral.
- [ ] This pull request has one subject. A formatting sweep bundled with a fix is
      harder to review and harder to revert.
- [ ] Fits [CONTRIBUTING.md].

<!--
Want a build of this branch to review? Add the "📦 upload artifacts" label and the
PR check attaches it to its run — the Artifacts box at the bottom of the run
summary. The build runs either way; only the upload depends on the label.

Found a security issue? Do not open a pull request in the open — see [SECURITY.md].
-->

[CONTRIBUTING.md]: CONTRIBUTING.md
[SECURITY.md]: https://github.com/BX-Team/.github/blob/master/SECURITY.md
