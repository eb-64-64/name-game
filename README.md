# Name Game

[Skip to installation instructions](#installation)

## Background

My family and I enjoy playing the name game together. The rules are fairly
simple: one person is designated as the "reader" for the round, and everyone
else writes any name (or even multiple names) on a small piece of paper and
hands it to the reader. Once everyone has submitted their names, the reader
reads all the names out loud, and the rest go around in a circle, trying to
guess who wrote which name, until all submitted names have been correctly
guessed. Then, somebody else is designated to be the reader, and another round
begins. Possible variations include choosing a theme for the round (e.g.,
famous actors, Disney characters, names beginning with A, etc.), or even
choosing a theme that doesn't revolve around names specifically (e.g.,
countries, cooking utensils, etc.; basically just the [Game of
Things](https://www.thegameofthings.com/)).

While we enjoyed playing the name game using paper, it was less than ideal for
two main reasons. First, because the reader could see who wrote what name based
on the handwriting, they had to sit around for a round, making it less fun for
them. Second, since the only time anyone heard the names was when they were
read, you had to try to remember all the names. While some people might find
this element of memory fun, my family preferred to have a list of names to
refer back to. Thus, this web app was born.

Instead of having to write their submission on paper to be read out by someone,
everyone could submit their names on their own device, and then a "display"
device could show all the submitted names (e.g., similar to
[Kahoot!](https://kahoot.it/) or [Jackbox](https://jackbox.tv/)). This solved
the two issues, as nobody has to sit out to be the reader, and a list is easily
available to everyone throughout the entire round.

## Installation

To run the project, you will need [Podman](https://podman.io/) and
[`just`](https://just.systems/man/en/introduction.html) installed. After that,
running the app is as simple as invoking

```sh
$ just build
```

to build the app's OCI image, and

```sh
$ just cold-start
```

to start the app. The app can be stopped with

```sh
$ just stop
```

and restarted with

```sh
$ just start
```

Finally, to clean up the app's containers and images, simply run

```sh
$ just clean
```

If there's an issue, the app's logs can be shown using

```sh
$ just app-logs
```

## Local development

To run the local development server, install the following softwares:

- [Bun](https://bun.com/) for the frontend;
- [Rust](https://rust-lang.org/) for the backend;
- [`mprocs`](https://github.com/pvolok/mprocs) to conveniently run the frontend
  dev server, backend server, and Valkey simultaneously;
- [Podman](https://podman.io/) to run Valkey during development; and
- [`docker-compose`](https://docs.docker.com/compose/) to run the Valkey image.

Then, the dev server can be started with

```sh
$ mprocs
```
