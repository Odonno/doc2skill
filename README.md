# doc2skill

> _"Why create skills with AI when you already have online documentation?  
> Use the #1 rule of programming: reuse what already exists."_

**doc2skill** turns package documentation into ready-to-use agent skills — no prompt engineering, no copy-pasting. Point it at a crate or NuGet package and get a clean Markdown skill file your AI agent can actually read.

---

## Install

### Cargo

```sh
cargo install doc2skill
```

### mise

```sh
mise use github:Odonno/doc2skill
```

> **Tip:** Add a short alias so you can type less every day:
>
> ```sh
> alias d2s = doc2skill
> ```

---

## Supported Languages

| Language       | Auto-detection                         | Package provider               | References                                           |
| -------------- | -------------------------------------- | ------------------------------ | ---------------------------------------------------- |
| **Rust**       | `Cargo.toml`                           | [crates.io](https://crates.io) | Traits, Structs, Enums, Functions, Constants, Macros |
| **C#**         | `*.csproj`, `Directory.Packages.props` | [NuGet](https://www.nuget.org) | N/A                                                  |
| **TypeScript** | `package.json`                         | [npm](https://www.npmjs.com)   | Classes, Interfaces, Functions, Variables            |

When a project file is found in the current directory, `doc2skill` automatically reads your dependencies and lets you pick which ones to generate skills for — no flags required.

---

## How to Use

### Generate a skill for a single package

```sh
doc2skill clap
```

```
name:        clap
version:     4.5.38
license:     MIT OR Apache-2.0
description: A simple to use, efficient, and full-featured Command Line Argument Parser
pages:       42 (41 references)
```

The skill is written to `.agents/skills/clap/SKILL.md` with reference pages under `.agents/skills/clap/references/`.

---

### Pin to a specific version

```sh
doc2skill clap@4.4.0
```

---

### Choose a language explicitly

```sh
doc2skill Newtonsoft.Json --language csharp
```

---

### Run interactively (no arguments)

```sh
doc2skill
```

If a `Cargo.toml` or `.csproj` is found, doc2skill lists your dependencies and lets you pick which skills to generate. Otherwise it drops into an interactive search prompt.

---

### Count tokens in generated skills

```sh
doc2skill --count
```

```
/.agents/skills
  /clap
    SKILL.md - 1 247 tokens
    references
      Arg.md        - 3 819 tokens
      Command.md    - 8 504 tokens
      Parser.md     - 612 tokens
  /serde
    SKILL.md - 983 tokens
    references
      Deserialize.md - 2 201 tokens
      Serialize.md   - 1 876 tokens
```

Token counts are highlighted in red when they exceed 5 000 tokens — a useful signal that a skill may be too large for some context windows.

Count tokens for a single skill:

```sh
doc2skill clap --count
```

---

## Output structure

```
.agents/skills/
└── clap/
    ├── SKILL.md          # main skill page
    └── references/
        ├── Arg.md
        ├── Command.md
        └── Parser.md
```

---

## License

[MIT](LICENSE)
