## What

<!-- One paragraph: the behaviour that changes and why. -->

## Checklist

- [ ] `bin/check` passes: fmt, clippy, tests, cargo deny, review dates, image build.
- [ ] New behaviour has a test in `tests/flow.rs` (protocol) or a unit test beside the code.
- [ ] No `unwrap`/`expect` outside tests; errors carry context.
- [ ] New configuration is in `src/config.rs`, the README and the site.
- [ ] New advisory ignores in `deny.toml` carry a reason and a review date.
