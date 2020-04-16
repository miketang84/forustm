#[macro_use] extern crate log;
#[macro_use]
extern crate serde_derive;
use std::env;
use env_logger;
use dotenv::dotenv;
use rusoda;
use serde;
use serde_json;


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


pub struct AppWebContext;
impl Key for AppWebContext {
    type Value = WebContext;
}

pub struct AppUser;
impl Key for AppUser {
    type Value = Ruser;
}

pub struct TtvIndex;
impl Key for TtvIndex {
   type Value = Arc<Mutex<tantivy_index::TantivyIndex>>;
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
    let ttv_index = match tantivy_index::init() {
       Ok(ttv_index) => {
	   Arc::new(Mutex::new(ttv_index))
       },
       Err(e) => {
	   panic!("{:?}", e);
       }
    };

    let addr = env::var("BINDADDR").expect("DBURL must be set");
    let port = env::var("BINDPORT").expect("REDISURL must be set").parse::<u32>().unwrap();
    let mut app = SapperApp::new();
    app.address(&addr)
	.port(port)
	.init_global(Box::new(move |req: &mut Request| {
	    req.ext_mut().insert::<TtvIndex>(ttv_index.clone());
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
