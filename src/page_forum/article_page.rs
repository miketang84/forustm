use sapper::{
    status,
    Request, 
    Response, 
    Result as SapperResult, 
    Error as SapperError, 
    Module as SapperModule,
    Router as SapperRouter};
use sapper_std::*;
use uuid::Uuid;

use crate::db;
use crate::cache;
use crate::envconfig;
// introduce macros
use sapper_std::res_html;
use crate::{
    AppWebContext,
    AppUser
};

use crate::dataservice::article::{
    Article,
    ArticleCreate,
    ArticleEdit
};
use crate::dataservice::section::Section;
use crate::dataservice::user::Ruser;

use crate::util::markdown_render;
use crate::middleware::permission_need_login;

struct CommentPaginator {
    total_comments: i32,
    total_page: i32,
    current_page: i32
}

pub struct ArticlePage;

impl ArticlePage {

    pub fn article_create_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);

        let section_id = t_param_parse_default!(params, "section_id", Uuid, Uuid::default());
        let sections = Section::forum_sections();

        web.insert("section_id", &section_id);
        web.insert("sections", &sections);


        res_html!("forum/new_article.html", web)
    }

    pub fn article_edit_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);
        let id = t_param_parse!(params, "id", Uuid);

        // get article object
        let article_r = Article::get_by_id(id);
        if article_r.is_err() {
            return res_400!(format!("no this artile: {}", id));
        }
        let article = article_r.unwrap();

        let sections = Section::forum_sections();

        web.insert("sections", &sections);
        web.insert("article", &article);

        res_html!("forum/edit_article.html", web)
    }

    pub fn article_delete_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);
        let id = t_param_parse!(params, "id", Uuid);

        // get article object
        let article_r = Article::get_by_id(id);
        if article_r.is_err() {
            return res_400!(format!("no this artile: {}", id));
        }
        let article = article_r.unwrap();

        web.insert("article", &article);

        res_html!("forum/delete_article.html", web)
    }
    
    pub fn article_detail_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();

        let params = get_query_params!(req);
        let id = t_param_parse!(params, "id", Uuid);
        let current_page = t_param_parse_default!(params, "current_page", i64, 1);

        let article_r = Article::get_by_id(id);
        if article_r.is_err() {
            return res_400!(format!("no this artile: {}", id));
        }
        let article = article_r.unwrap();

        let author_r = Ruser::get_user_by_id(article.author_id);
        if author_r.is_err() {
            return res_400!(format!("no this author: {}", article.author_id));
        }
        let author = author_r.unwrap();

        let mut is_author = false;
        let mut is_admin = false;
        let mut is_login = false;
        match ext_type!(req, AppUser) {
            Some(user) => {
                if article.author_id == user.id {
                    is_author = true;
                }
                if user.role >= 9 {
                    is_admin = true;
                }

                is_login = true;
                web.insert("is_login", &is_login);
                web.insert("user", &user);
            },
            None => {}
        }

        let ncpp = envconfig::get_int_item("NUMBER_COMMENT_PER_PAGE");
        // retrieve comments belongs to this article, and calculate its paginator
        let total_item = Article::get_comments_count_belong_to_this(id);
        let total_page = ((total_item -1) / ncpp) as i64 + 1;
        let comments = Article::get_comments_paging_belong_to_this(id, current_page);

        let viewtimes = Article::get_viewtimes(article.id);
        Article::increase_viewtimes(article.id);

        web.insert("article", &article);
        web.insert("author", &author);
        web.insert("comments", &comments);
        web.insert("current_page", &current_page);
        web.insert("total_item", &total_item);
        web.insert("total_page", &total_page);
        web.insert("is_author", &is_author);
        web.insert("is_admin", &is_admin);
        web.insert("viewtimes", &viewtimes);

        res_html!("forum/article.html", web)
    }

    pub fn article_create(req: &mut Request) -> SapperResult<Response> {
        let params = get_form_params!(req);
        let section_id = t_param_parse_default!(params, "section_id", Uuid, Uuid::default());
        let title = t_param!(params, "title").to_owned();
        let tags = t_param!(params, "tags").to_owned();
        let raw_content = t_param!(params, "raw_content");
        let stype = t_param_parse_default!(params, "stype", i32, 0);

        let content = markdown_render(raw_content);
        let user = ext_type!(req, AppUser).unwrap();

        let raw_content = raw_content.to_owned();
        let article_create = ArticleCreate {
            title,
            tags,
            section_id,
            author_id: user.id,
            raw_content,
            content,
            stype,
            status: 0,
        };

        match article_create.insert() {
            Ok(article) => {
                res_redirect!(format!("/article?id={}", article.id))
            },
            Err(_) => {
                res_500!("article create error.")
            }
        }  
     }

    pub fn article_edit(req: &mut Request) -> SapperResult<Response> {
        let params = get_form_params!(req);
        let id = t_param_parse!(params, "id", Uuid);
        let section_id = t_param_parse!(params, "section_id", Uuid);
        let title = t_param!(params, "title").to_owned();
        let tags = t_param!(params, "tags").to_owned();
        let raw_content = t_param!(params, "raw_content");

        let content = markdown_render(raw_content);
        let raw_content = raw_content.to_owned();

        let article_edit = ArticleEdit {
            id,
            section_id,
            title,
            tags,
            raw_content,
            content,
        };

        match article_edit.update() {
            Ok(article) => {
                res_redirect!(format!("/article?id={}", article.id))
            },
            Err(_) => {
                res_500!("article edit error.")
            }
        }  
    }

    pub fn article_delete(req: &mut Request) -> SapperResult<Response> {
        let params = get_form_params!(req);
        let article_id = t_param_parse!(params, "article_id", Uuid);
        let section_id = t_param_parse!(params, "section_id", Uuid);

        match Article::delete_by_id(article_id) {
            Ok(article) => {
                res_redirect!(format!("/section?id={}", section_id))
            },
            Err(_) => {
                res_500!("article delete error.")
            }
        }  
    }



    // Blog Area
    pub fn blog_article_create_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);

        let is_in_blog = true;
        web.insert("is_in_blog", &is_in_blog);

        res_html!("forum/new_article.html", web)
    }

    pub fn blog_article_edit_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);
        let id = t_param_parse!(params, "id", Uuid);

        let is_in_blog = true;
        web.insert("is_in_blog", &is_in_blog);

        // get article object
        let article = Article::get_by_id(id);
        if article.is_err() {
            return res_400!(format!("no this artile: {}", id));
        }
        let article = article.unwrap();

        web.insert("article", &article);

        res_html!("forum/edit_article.html", web)
    }

    pub fn blog_article_create(req: &mut Request) -> SapperResult<Response> {
        let params = get_form_params!(req);

        let title = t_param!(params, "title").to_owned();
        let tags = t_param!(params, "tags").to_owned();
        let raw_content = t_param!(params, "raw_content");
        let stype = t_param_parse_default!(params, "stype", i32, 1);
        let user = ext_type!(req, AppUser).unwrap();
        let section_id = Section::get_by_suser(user.id).unwrap().id;

        let content = markdown_render(raw_content);
        let raw_content = raw_content.to_owned();
        let article_create = ArticleCreate {
            title,
            tags,
            section_id,
            author_id: user.id,
            raw_content,
            content,
            stype,
            status: 0,
        };

        match article_create.insert() {
            Ok(article) => {
                res_redirect!(format!("/article?id={}", article.id))
            },
            Err(_) => {
                res_500!("article create error.")
            }
        }  
     }

     pub fn blog_article_edit(req: &mut Request) -> SapperResult<Response> {

        let params = get_form_params!(req);
        let id = t_param_parse!(params, "id", Uuid);
        let title = t_param!(params, "title").to_owned();
        let tags = t_param!(params, "tags").to_owned();
        let raw_content = t_param!(params, "raw_content");
        let user = ext_type!(req, AppUser).unwrap();
        let section_id = Section::get_by_suser(user.id).unwrap().id;

        let content = markdown_render(raw_content);
        let raw_content = raw_content.to_owned();

        let article_edit = ArticleEdit {
            id,
            section_id,
            title,
            tags,
            raw_content,
            content,
        };

        match article_edit.update() {
            Ok(article) => {
                res_redirect!(format!("/article?id={}", article.id))
            },
            Err(_) => {
                res_500!("article edit error.")
            }
        }  
    }

}


impl SapperModule for ArticlePage {
    fn before(&self, req: &mut Request) -> SapperResult<()> {
        let (path, _) = req.uri();
        if envconfig::get_int_item("CACHE") == 1 {
            if &path == "/article" {
                let params = get_query_params!(req);
                let article_id = t_param!(params, "id");
                let current_page = t_param_parse_default!(params, "current_page", i64, 1);
                let part_key = article_id.to_string() + ":" + &current_page.to_string();
                if cache::cache_is_valid("article", &part_key) {
                    let cache_content = cache::cache_get("article", &part_key);
                    return res_html_before!(cache_content);
                }
            }
        }

        match permission_need_login(req) {
            Ok(_) => {
                // pass, nothing need to do here
            },
            Err(info) => {
                return Err(SapperError::Custom("no permission".to_string()));
            }
        }

        Ok(())
    }

    fn after(&self, req: &Request, res: &mut Response) -> SapperResult<()> {
        let res_status = res.status();
        if res_status == status::Ok || res_status == status::Found {
            let (path, _) = req.uri();
            if &path == "/s/article/create" 
                || &path == "/s/article/edit" 
                || &path == "/s/article/delete" 
                || &path == "/s/blogarticle/create" 
                || &path == "/s/blogarticle/edit" {
            
                cache::cache_set_invalid("index", "index");
            }

            // check other urls
            if &path == "/s/article/create" 
                || &path == "/s/article/edit"
                || &path == "/s/article/delete" {
                
                let params = get_form_params!(req);
                let section_id = t_param_parse!(params, "section_id", Uuid);

                let napp = envconfig::get_int_item("NUMBER_ARTICLE_PER_PAGE");
                let n = Section::get_articles_count_belong_to_this(section_id);
                let total_page = ((n -1) / napp) as i64 + 1;

                for i in 1..=total_page {
                    let part_key = section_id.to_string() + ":" + &i.to_string();
                    cache::cache_set_invalid("section", &part_key);
                }
            }

            if &path == "/s/blogarticle/create" 
                || &path == "/s/blogarticle/edit" {
                let user = ext_type!(req, AppUser).unwrap();
                let section_id = Section::get_by_suser(user.id).unwrap().id;

                let napp = envconfig::get_int_item("NUMBER_ARTICLE_PER_PAGE");
                let n = Section::get_articles_count_belong_to_this(section_id);
                let total_page = ((n -1) / napp) as i64 + 1;

                for i in 1..=total_page {
                    let part_key = section_id.to_string() + ":" + &i.to_string();
                    cache::cache_set_invalid("section", &part_key);
                }
            }

            if &path == "/article" {
                let params = get_query_params!(req);
                let article_id = t_param!(params, "id");
                let current_page = t_param_parse_default!(params, "current_page", i64, 1);
                let part_key = article_id.to_string() + ":" + &current_page.to_string();
                if !cache::cache_is_valid("article", &part_key) {
                    cache::cache_set("article", &part_key, res.body());
                }
            }

            if &path == "/s/article/edit" 
                || &path == "/s/blogarticle/edit"  {
                let params = get_form_params!(req);
                let article_id = t_param!(params, "id");

                cache::cache_set_invalid("article", article_id);
            }

        }

        Ok(())
    }

    fn router(&self, router: &mut SapperRouter) -> SapperResult<()> {
        router.get("/article", Self::article_detail_page);

        router.get("/p/article/create", Self::article_create_page);
        router.get("/p/article/edit", Self::article_edit_page);
        router.get("/p/article/delete", Self::article_delete_page);
        router.post("/s/article/create", Self::article_create);
        router.post("/s/article/edit", Self::article_edit);
        router.post("/s/article/delete", Self::article_delete);

        router.get("/p/blogarticle/create", Self::blog_article_create_page);
        router.get("/p/blogarticle/edit", Self::blog_article_edit_page);
        router.post("/s/blogarticle/create", Self::blog_article_create);
        router.post("/s/blogarticle/edit", Self::blog_article_edit);

        Ok(())
    }
}


