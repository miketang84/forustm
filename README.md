## Forustm

Forustm is a forum written in Rust.

#### External Dependencies

Now, we use postgresql as main db, to store all things, and use redis to store user login session key.

- Redis, version > 2
- Postgresql, version > 9

and you will find ./schema/schema.sql, you need use pg client, such as `psql`, to make a new empty db in postgresql before bootuping forustm bin.

#### Structure

This forum is developed by [Sapper](https://github.com/daogangtang/sapper), which is a rapid web developing framework (which is based on syncronized hyper v0.10.26 now, but later will update to latest async/await branch).

Although it is not well documented now, it ran as a community forum for three years (wow :) )

But I will surely supply more documentations on it, in fact I am waiting the release of `async-std` and correspoding hyper branch version, or other http server crate version.

#### Features

- Section CRUD
- Article CRUD
- User blog
- User blog planet
- I18n
- Fulltext searching based on tantivy (but now commented, need to update to latest tantivy version)
- User system oauthed by github.com (this is the only user system currently, we don't supply opening registration)
- Tera rendering engine
- Markdown text support for article content and comment content
- Rss subscribing

We write this project for:

1. Making a forum for rust community (must use rust to implement it)
2. Giving a practical example for sapper project
3. Keeping code clean and easy to learn

#### Configuration

You need an `.env` file in your project directory, whose content as like bellow:

```
DBURL=postgres://postgres:pwd@localhost/forustm
REDISURL=redis://127.0.0.1/0
BINDADDR=127.0.0.1
BINDPORT=8081
RUSODA_LANG=en
#HOST_DOMAIN=https://rust.cc
#HOST_DOMAIN=http://127.0.0.1:8080
NUMBER_ARTICLE_PER_PAGE=20
NUMBER_COMMENT_PER_PAGE=20
CACHE=0
```

#### Bootup

```
cargo build
cargo run --bin page_forum_bin
```

and you will see a boot up server which is bounded to the port you configured just now.

#### Operating

And then, you can operate everything in the browser.

#### Good Lucky

This is not a detailed documentation, so, forgive me, will do more on it.

#### Future Plan

Rust community should own its own full-featured forum application, the long aim of this project, is to replace the officail [Rust users forum](https://users.rust-lang.org/) keeped by Rust officail team.

#### Referrences

- [rust.cc](https://rust.cc)
- [Rustforce](https://rustforce.net)
- [Rust Users](https://users.rust-lang.org/)
