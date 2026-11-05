# Introduction

Kwark is a terminal-based text-editor "framework". By framework, I mean that kwark doesn't just "run"
on its own, you **must** use it as a library in your own rust "project" (kwark calls these `flavors`).

This is a very different approach to lots of "applications" that come around in the editing community.
However, I think that this model works incredibly well, as I am able to accomplish a lot of integration
with users that I wouldn't be able to otherwise.

## About The Book
This book is a **hand**-written guide as to how kwark works, and how users can best learn to interact with this tool.
**NONE** of this book is AI-written or AI-assisted, it all comes from my stupid little brain, so hopefully that makes
it a little more fun to read :D.

### Code Snippets
Most, if not all code snippets come from the [examples](https://github.com/TheEmeraldBee/kwark/tree/main/kwark/examples) folder on `kwark`'s github, so if you see code, and want to try it out,
look there first. All code is written and carfully commented, and is directly *embedded* into this book from that folder, so it
should always be up to date with the *git* version of kwark.

### I found an issue in the book
If you see any issues, please create a **non-ai** written PR *or* Issue
inside of the [kwark](https://github.com/TheEmeraldBee/kwark/tree/main) repository, anything AI generated
will be ignored and thrown out immediately.

## About the Editor

### Views on AI
I know AI is a very touchy subject, and I'll be blunt, I use AI **heavily** at work, and hate the way it communicates more than anything,
however, I also see how powerful it is, and how much it makes me be able to focus on the parts of
coding I like (type-system shenanigans, compilers, look and feel of the code contracts, etc), and simplify the parts I don't like.

So, in saying this, **yes**, kwark has some AI written code, however, ever line of code in this project
I treat like my own, everything inside is understood and known about by humans, and that will never change.

If you feel the need to create a PR for this project, I **WILL NOT** accept an AI-Generated pull request that hasn't
been thouroughly reviewed and understood by you, this may mean I will ask you about how some things were done, or why.
I hold my right of this being my project, to deem the code written AI-slop, and if it is, your PR will be ignored.

**HOWEVER** AI-Slop and AI-assistance are 2 very different things. If you are up front about AI use, and you understand
the code that was written, and the comments and doc-comments are human-written so other humans can understand, more than
likely your PR will be looked at and cared abbout.

### Goals of this Project
- Be a fully configurable, well featured text-editor
- Support rust as both a plugin-manager and configuration system
- Be easy to understand and customize to your own usecases

### Why Rust
Well, for starters, rust is a very fun language to write in. In my personal opinion, I don't care all that much about it being "fast", or "memory-safe" *(though those things are awesome)*.
I care much more about how fun a language is to use, and how much effort I would want to put in to know that language better, and rust hit both of those points for me.

### Why No Scripting Language
Rust is an amazing language, and I like using it to do things. As well, making `kwark` a framework was not a light choice.
- I have gone through multiple iterations of this project, before, it was named `Kerbin`, and I wrote a light "shell-style" language for it, and it just felt too out of place.
- I have messed with using `rune` for projects, but again, little editor support (no lsp), and it was just not worth learning.

So, after multiple iterations, I asked myself, why not just use rust, I had tried it in Kerbin, but it failed because I was trying to do it over FFI (not easy while keeping the type system nice).
This lead me to the decision of turning my new implementation of my editor (`kwark`) into a framework, and having your configuration just be the application that pulls in the framework.

This gave me access to all of the type-system guarentees I was missing from `lua`, the strong editor-integrations I was missing in `rune`, and the nice-feeling of use that none of them gave me.

#### What about compile times?
Yes, I know that rust compile-times can get quite slow, and that is definitely a worry of mine, however, rust's incremental compilation is quite fast, and is able to skip a lot of work
most of the time, so see for yourself, but I rarely see myself changing the configuration enough to care about <10 seconds of compilation here and there.
