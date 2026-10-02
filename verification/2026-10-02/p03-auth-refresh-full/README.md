All301 login tests passed on source42e3899a (8,949 files), with zero skips or retries. Both actual test binaries were observed: new integration303fccbe and focused login a87bdcd2. All five reload and four refresh cases executed; strict cleanup, source and final OOM guards passed.

Execution was serialized under grouped CLI+login features. All305 pre-exec network-guard flags were false; independent proof that occupied-port guards never returned early is unavailable. Compiled CLI882ecabb tests did not execute.

This is current-source package evidence. Provider/lint and production/UI gates remain separate. Production CLI78d is still archive-only and unavailable; previous0742 test results retain their original source identity.
