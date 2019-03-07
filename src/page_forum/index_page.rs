use sapper::{
    status,
    Request, 
    Response, 
    Result as SapperResult, 
    Error as SapperError, 
    Module as SapperModule,
    Router as SapperRouter};
use sapper_std::*;

use crate::db;
// introduce macros
use sapper_std::res_html;
use crate::AppWebContext;
use crate::cache;
use crate::rss;

use crate::envconfig;
use crate::dataservice::article::Article;
use crate::dataservice::section::Section;

use crate::TtvIndex;
use crate::tantivy_index::{DocFromIndex, Doc2Index};


pub struct IndexPage;

impl IndexPage {

    pub fn index(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();

        let napp = envconfig::get_int_item("NUMBER_ARTICLE_PER_PAGE");
        let articles = Article::get_latest_articles(napp);

        let blog_articles = Article::get_latest_blog_articles(napp);

        // get all configured index displaying sections
        // and latest commented three articles 
        let sections = Section::forum_sections();

        web.insert("articles", &articles);
        web.insert("blog_articles", &blog_articles);
        web.insert("sections", &sections);

        res_html!("forum/index.html", web)
    }

    pub fn rss_xml(req: &mut Request) -> SapperResult<Response> {
        let rss_string = rss::make_rss_feed();

        res_xml_string!(rss_string)
    }

    pub fn search_query_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();

        let params = get_query_params!(req);
        let q = t_param_default!(params, "q", "");

        let mut docs: Vec<DocFromIndex> = Vec::new();
        if q != "" {
            let ttv_index = ext_type!(req, TtvIndex).unwrap().lock().unwrap();
            docs = ttv_index.query(q).unwrap();

            println!("{:?}", docs);
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
        let mut ttv_index = ext_type!(req, TtvIndex).unwrap().lock().unwrap();

        let articles = Article::get_latest_full_articles(20);

        for article in articles {
            let doc2index = Doc2Index {
                article_id: article.id.to_string(),
                title: article.title,
                content: article.raw_content
            };
            ttv_index.add_doc(doc2index).unwrap();
        }

        println!("Make index test finished.");

        res_redirect!("/search")
    }

    

}


impl SapperModule for IndexPage {
    fn before(&self, req: &mut Request) -> SapperResult<()> {
        let (path, _) = req.uri();
        if &path == "/" {
            if cache::cache_is_valid("index", "index") {
                let cache_content = cache::cache_get("index", "index");
                
                splog(req, status::Ok).unwrap();
                return res_html_before!(cache_content);
            }
        }
        
        Ok(())
    }

    fn after(&self, req: &Request, res: &mut Response) -> SapperResult<()> {
        let (path, _) = req.uri();

        if envconfig::get_int_item("CACHE") == 1 {
            if &path == "/" {
                if !cache::cache_is_valid("index", "index") {
                    cache::cache_set("index", "index", res.body());
                }
            }
        }

        Ok(())
    }


    fn router(&self, router: &mut SapperRouter) -> SapperResult<()> {
        router.get("/", Self::index);
        router.get("/rss", Self::rss_xml);
        router.get("/search", Self::search_query_page);
        router.post("/search", Self::search_query);

        router.get("/makeindex", Self::makeindex);


        Ok(())
    }
}


