use sapper::{
    status,
    Request,
    Response,
    Result as SapperResult,
    Error as SapperError,
    Module as SapperModule,
    Router as SapperRouter};
use sapper_std::*;
use log::info;

use crate::db;
// introduce macros
use sapper_std::res_html;
use crate::{AppWebContext, AppUser};
use crate::cache;
use crate::rss;

use crate::envconfig;
use crate::dataservice::article::Article;
use crate::dataservice::section::Section;

use crate::{TanIndexTx, TanQueryRx};
use crate::tantivy_index::{DocFromIndexOuter, Doc2Index, TanAction};
use crate::middleware::{
    permission_need_be_admin,
    check_cache_switch
};

pub struct IndexPage;

impl IndexPage {

    pub fn index(req: &mut Request) -> SapperResult<Response> {
        let mut web = get_ext_owned!(req, AppWebContext).unwrap();

        let napp = envconfig::get_int_item("NUMBER_ARTICLE_PER_PAGE");
        let articles = Article::get_latest_articles(napp);

        let reply_articles = Article::get_latest_reply_articles(napp);

        let blog_articles = Article::get_latest_blog_articles(napp);

        // get all configured index displaying sections
        // and latest commented three articles
        let sections = Section::forum_sections();

        web.insert("articles", &articles);
        web.insert("reply_articles", &reply_articles);
        web.insert("blog_articles", &blog_articles);
        web.insert("sections", &sections);

        res_html!("forum/index.html", web)
    }

    pub fn rss_xml(req: &mut Request) -> SapperResult<Response> {
        let rss_string = rss::make_rss_feed();

        res_xml_string!(rss_string)
    }

    pub fn search_query_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = get_ext_owned!(req, AppWebContext).unwrap();

        let params = get_query_params!(req);
        let q = t_param_default!(params, "q", "");

        let mut docs: Vec<DocFromIndexOuter> = Vec::new();
        if q != "" {
            let tan_index = get_ext!(req, TanIndexTx).unwrap();
            // send query directive
            tan_index.send((TanAction::Query, q.to_string(), None)).unwrap();

            let tan_query = get_ext!(req, TanQueryRx).unwrap();
            // block receiving
            docs = tan_query.recv().unwrap();
        }

        web.insert("docs", &docs);
        web.insert("q", q);

        res_html!("forum/search_result.html", web)
    }

    pub fn search_query(req: &mut Request) -> SapperResult<Response> {
        let params = get_form_params!(req);
        let q = t_param!(params, "q");

        res_redirect!(format!("/search?q={}", q))
    }

    pub fn makeindex(req: &mut Request) -> SapperResult<Response> {
        permission_need_be_admin(req)?;

        let tan_index = get_ext!(req, TanIndexTx).unwrap();

        let articles = Article::get_all_articles();

        for article in articles {
            let doc2index = Doc2Index {
                article_id: article.id.to_string(),
                created_time: article.created_time.timestamp().to_string(),
                title: article.title,
                content: article.raw_content
            };
            // send query directive
            tan_index.send((TanAction::Add, "".to_string(), Some(doc2index))).unwrap();
        }

        info!("Make index test finished.");

        res_redirect!("/search")
    }

    pub fn acknowledgement(req: &mut Request) -> SapperResult<Response> {
        let mut web = get_ext_owned!(req, AppWebContext).unwrap();

        res_html!("forum/acknowledgement.html", web)
    }

    pub fn latest_articles_paging(req: &mut Request) -> SapperResult<Response> {
        let mut web = get_ext_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);

        let current_page = t_param_parse_default!(params, "current_page", i64, 1);

        let mut is_admin = false;
        let mut is_login = false;
        match get_ext!(req, AppUser) {
            Some(user) => {
                if user.role >= 9 {
                    is_admin = true;
                }

                is_login = true;
                web.insert("is_login", &is_login);
                web.insert("user", &user);
            },
            None => {}
        }

        let napp = envconfig::get_int_item("BIG_NUMBER_ARTICLE_PER_PAGE");
        let total_item = Article::get_all_section_articles_count();
        let total_page = ((total_item - 1) / napp) as i64 + 1;

        let articles = Article::get_latest_articles_paging(current_page-1, napp);

        web.insert("is_admin", &is_admin);
        web.insert("total_item", &total_item);
        web.insert("total_page", &total_page);
        web.insert("current_page", &current_page);
        web.insert("articles", &articles);
        web.insert("this_page_url", "latest_articles_paging");
        web.insert("s_title", "Latest Articles");

        res_html!("forum/article_list_paging.html", web)
    }

    pub fn latest_reply_articles_paging(req: &mut Request) -> SapperResult<Response> {
        let mut web = get_ext_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);

        let current_page = t_param_parse_default!(params, "current_page", i64, 1);

        let mut is_admin = false;
        let mut is_login = false;
        match get_ext!(req, AppUser) {
            Some(user) => {
                if user.role >= 9 {
                    is_admin = true;
                }

                is_login = true;
                web.insert("is_login", &is_login);
                web.insert("user", &user);
            },
            None => {}
        }

        let napp = envconfig::get_int_item("BIG_NUMBER_ARTICLE_PER_PAGE");
        let total_item = Article::get_all_section_articles_count();
        let total_page = ((total_item - 1) / napp) as i64 + 1;

        let articles = Article::get_latest_reply_articles_paging(current_page-1, napp);

        web.insert("is_admin", &is_admin);
        web.insert("total_item", &total_item);
        web.insert("total_page", &total_page);
        web.insert("current_page", &current_page);
        web.insert("articles", &articles);
        web.insert("this_page_url", "latest_reply_articles_paging");
        web.insert("s_title", "Latest Articles On Reply");

        res_html!("forum/article_list_paging.html", web)
    }

    pub fn latest_blog_articles_paging(req: &mut Request) -> SapperResult<Response> {
        let mut web = get_ext_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);

        let current_page = t_param_parse_default!(params, "current_page", i64, 1);

        let mut is_admin = false;
        let mut is_login = false;
        match get_ext!(req, AppUser) {
            Some(user) => {
                if user.role >= 9 {
                    is_admin = true;
                }

                is_login = true;
                web.insert("is_login", &is_login);
                web.insert("user", &user);
            },
            None => {}
        }

        let napp = envconfig::get_int_item("BIG_NUMBER_ARTICLE_PER_PAGE");
        let total_item = Article::get_all_blog_articles_count();
        let total_page = ((total_item - 1) / napp) as i64 + 1;

        let articles = Article::get_latest_blog_articles_paging(current_page-1, napp);

        web.insert("is_admin", &is_admin);
        web.insert("total_item", &total_item);
        web.insert("total_page", &total_page);
        web.insert("current_page", &current_page);
        web.insert("articles", &articles);
        web.insert("this_page_url", "latest_blog_articles_paging");
        web.insert("s_title", "Latest Notes");

        res_html!("forum/article_list_paging.html", web)
    }

}


impl SapperModule for IndexPage {
    fn before(&self, req: &mut Request) -> SapperResult<()> {
        let (path, _) = req.uri();
        if check_cache_switch(req) {
            if &path == "/" {
                if cache::cache_is_valid("index", "index") {
                    let cache_content = cache::cache_get("index", "index");

                    splog(req, status::Ok).unwrap();
                    return res_html_before!(cache_content);
                }
            }
        }

        Ok(())
    }

    fn after(&self, req: &Request, res: &mut Response) -> SapperResult<()> {
        let (path, _) = req.uri();
        if &path == "/" {
            if !cache::cache_is_valid("index", "index") {
                cache::cache_set("index", "index", res.body());
            }
        }

        Ok(())
    }


    fn router(&self, router: &mut SapperRouter) -> SapperResult<()> {
        router.get("/", Self::index);
        router.get("/latest_articles_paging", Self::latest_articles_paging);
        router.get("/latest_reply_articles_paging", Self::latest_reply_articles_paging);
        router.get("/latest_blog_articles_paging", Self::latest_blog_articles_paging);

        router.get("/rss", Self::rss_xml);
        router.get("/search", Self::search_query_page);
        router.post("/search", Self::search_query);
        router.get("/acknowledgement", Self::acknowledgement);

        // need to be limited call by admin only
        router.get("/makeindex", Self::makeindex);


        Ok(())
    }
}
