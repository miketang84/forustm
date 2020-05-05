#[macro_use] extern crate log;
#[macro_use]
extern crate serde_derive;
use std::{env, thread};
use env_logger;
use dotenv::dotenv;
use rusoda;
use serde;
use serde_json;
use crossbeam::channel;

use std::sync::{
    Arc,
    Mutex,
};

//#[macro_use] extern crate sapper_std;
use sapper::{
    App as SapperApp,
    Armor as SapperArmor,
    Result as SapperResult,
    Request,
    Response,
    Key
};
use sapper_std::*;

use rusoda::envconfig;
use rusoda::db;
use rusoda::cache;
use rusoda::dataservice;
use rusoda::util;
use rusoda::github_utils;
use rusoda::i18n;
use rusoda::web_filters;
use rusoda::rss;

mod middleware;
mod tantivy_index;

// include page modules
mod page_forum;

use self::dataservice::user::Ruser;
use self::tantivy_index::{
    DocFromIndexOuter, TanAction, Doc2Index,
};



pub struct AppWebContext;
impl Key for AppWebContext {
    type Value = WebContext;
}

pub struct AppUser;
impl Key for AppUser {
    type Value = Ruser;
}

pub struct TanIndexTx;
impl Key for TanIndexTx {
   type Value = channel::Sender<(TanAction, String, Option<Doc2Index>)>;
}
pub struct TanQueryRx;
impl Key for TanQueryRx {
   type Value = channel::Receiver<Vec<DocFromIndexOuter>>;
}


// define global smock
struct PageForum;

impl SapperArmor for PageForum {
    fn before(&self, req: &mut Request) -> SapperResult<()> {
        // define cookie prefix
        sapper_std::init(req, Some("rusoda_session"))?;
        // init web instance state
        let mut web = WebContext::new();
        // we can add something to web
        match req.ext().get::<SessionVal>() {
            Some(cookie) => {
                // using this cookie to retreive user instance
                match Ruser::get_user_by_cookie(&cookie) {
                    Ok(user) => {
                        if user.status == 0 {
                            web.insert("user", &user);
                            req.ext_mut().insert::<AppUser>(user);
                        }
                    },
                    Err(_) => {}
                }
            },
            None => {}
        }

        // insert it to req
        req.ext_mut().insert::<AppWebContext>(web);

        Ok(())
    }

    fn after(&self, req: &Request, res: &mut Response) -> SapperResult<()> {
        sapper_std::finish(req, res)?;
        Ok(())
    }
}

fn main () {
    env_logger::init();
    dotenv().ok();
    //
    web_filters::register_web_filters();

    // create first pair channel: send directive
    let (tan_index_tx, tan_index_rx) = channel::unbounded::<(TanAction, String, Option<Doc2Index>)>();
    // create query result pair channel
    let (tan_query_tx, tan_query_rx) = channel::unbounded::<Vec<DocFromIndexOuter>>();

    tantivy_index::run_tantivy(tan_index_rx, tan_query_tx);

    let addr = env::var("BINDADDR").expect("DBURL must be set");
    let port = env::var("BINDPORT").expect("REDISURL must be set").parse::<u32>().unwrap();
    let mut app = SapperApp::new();
    app.address(&addr)
        .port(port)
        .init_global(Box::new(move |req: &mut Request| {
            req.ext_mut().insert::<TanIndexTx>(tan_index_tx.clone());
            req.ext_mut().insert::<TanQueryRx>(tan_query_rx.clone());

            Ok(())
        }))
        .with_armor(Box::new(PageForum))
        .add_module(Box::new(page_forum::index_page::IndexPage))
        .add_module(Box::new(page_forum::user_page::UserPage))
        .add_module(Box::new(page_forum::section_page::SectionPage))
        .add_module(Box::new(page_forum::article_page::ArticlePage))
        .add_module(Box::new(page_forum::comment_page::CommentPage))
        .static_file_service(true);

    println!("Start listen on http://{}:{}", addr, port);
    app.run_http();

}
