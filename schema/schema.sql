-- Your SQL goes here

create extension pgcrypto;

CREATE TABLE ruser (
  id uuid primary key default gen_random_uuid(),
  account VARCHAR unique NOT NULL,
  password VARCHAR NOT NULL,
  salt VARCHAR NOT NULL,
  nickname VARCHAR NOT NULL,
  avatar VARCHAR,
  wx_openid VARCHAR,
  say VARCHAR,
  signup_time timestamp not null default current_timestamp,
  role smallint not null default 0,
  status smallint not null default 0,
  github varchar 
);

create index user_account on ruser (account);

insert into ruser (account, password, salt, role, nickname) values
('admin@admin.com', '325c162157dea106ce5bacc705c4929e4ec526a0290bfaba2dcbbf18103c7c2b', 'MKsiaw', 9, 'admin');

CREATE TABLE section (
  id uuid primary key default gen_random_uuid(),
  title VARCHAR NOT NULL,
  description VARCHAR NOT NULL,
  stype INTEGER NOT NULL,
  suser uuid references ruser (id),
  created_time timestamp not null default current_timestamp,
  status smallint not null default 0,
  weight double precision not null default 0.0                 -- new
);

CREATE TABLE article (
  id uuid primary key default gen_random_uuid(),
  title VARCHAR NOT NULL,
  raw_content VARCHAR NOT NULL,
  content VARCHAR NOT NULL,
  section_id uuid references section (id) not null,
  author_id uuid references ruser (id) not null,
  tags VARCHAR NOT NULL,
  extlink VARCHAR NOT NULL default '',
  stype INTEGER NOT NULL,
  created_time timestamp not null default current_timestamp,
  status smallint not null default 0,
  updated_time timestamp default now()::timestamp + '-1 year'
);

CREATE TABLE comment (
  id uuid primary key default gen_random_uuid(),
  raw_content VARCHAR NOT NULL,                     -- new
  content VARCHAR NOT NULL,
  article_id uuid references article (id) not null,
  author_id uuid references ruser (id) not null,
  created_time timestamp not null default current_timestamp,
  status smallint not null default 0
);

CREATE TABLE articleweight (
  id uuid primary key default gen_random_uuid(),
  section_id uuid references section (id) not null,
  article_id uuid references article (id) not null,
  weight double precision not null default 0.0,
  created_time timestamp not null default current_timestamp
);

