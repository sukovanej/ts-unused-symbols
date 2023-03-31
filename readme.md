# ts-unused-symbols

*Under development*

Find unused exports in a typescript codebase.

- Supports yarn workspaces.
- Analyzes typescript directly.
- Fast.

## How it works

The analyzer uses [swc parser](https://github.com/swc-project/swc) under the hood. The analysis takes
into account monorepo structure by introspecting `package.json`. The analysis parses all the typescript
source files in the codebase and marks exported and imported symbols. To identify usages between packages
in the monorepo, it introspects build folders and uses source-maps to correctly map imports onto source
ts files. Therefore, to make the analysis work correctly, all the packages need to be built and more
importantly, they need to be built with `"declarationMap": true`.
