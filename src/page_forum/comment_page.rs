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
// introduce macros
use sapper_std::res_html;
use crate::{
    AppWebContext,
    AppUser
};

use crate::dataservice::article::Article;
use crate::dataservice::comment::{
    Comment,
    CommentCreate,
    CommentEdit
};

use crate::util::markdown_render;
use crate::middleware::permission_need_login;
use crate::envconfig;


pub struct CommentPage;

impl CommentPage {

    pub fn comment_new_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);
        let article_id = t_param_parse!(params, "article_id", Uuid);
        if t_has_param!(params, "reply_comment_id") {
            let reply_comment_id = t_param_parse!(params, "reply_comment_id", Uuid);
            match Comment::get_by_id(reply_comment_id) {
                Ok(comment) => {
                    web.insert("reply_comment", &comment);

                    match Article::get_by_id(article_id) {
                        Ok(article) => {
                            web.insert("article", &article);
                            return res_html!("forum/new_comment.html", web);
                        },
                        Err(_) => {
                            return res_500!("no this article.");
                        }
                    }
                },
                Err(_) => {
                    return res_500!("no this reply comment.");
                }
            }
        }
        else {
            match Article::get_by_id(article_id) {
                Ok(article) => {
                    web.insert("article", &article);
                    return res_html!("forum/new_comment.html", web);
                },
                Err(_) => {
                    return res_500!("no this article.");
                }
            }
        }
    }

    pub fn comment_edit_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);
        let article_id = t_param_parse!(params, "article_id", Uuid);
        let comment_id = t_param_parse!(params, "comment_id", Uuid);

        match Article::get_by_id(article_id) {
            Ok(article) => {
                match Comment::get_by_id(comment_id) {
                    Ok(comment) => {
                        web.insert("article", &article);
                        web.insert("comment", &comment);
                        return res_html!("forum/edit_comment.html", web);
                    },
                    Err(_) => {
                        return res_500!("no this comment.");
                    }
                }
            },
            Err(_) => {
                return res_500!("no this article.");
            }
        }
    }

    pub fn comment_delete_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);
        let comment_id = t_param_parse!(params, "id", Uuid);

        match Comment::get_by_id(comment_id) {
            Ok(comment) => {
                web.insert("comment", &comment);
                return res_html!("forum/delete_comment.html", web);
            },
            Err(_) => {
                return res_500!("no this comment.");
            }
        }
    }

    pub fn comment_new(req: &mut Request) -> SapperResult<Response> {
        let params = get_form_params!(req);
        let article_id = t_param_parse!(params, "article_id", Uuid);
        let raw_content = t_param!(params, "raw_content");

        let content = markdown_render(raw_content);
        let raw_content = raw_content.to_owned();

        let user = ext_type!(req, AppUser).unwrap();
        let author_id = user.id;

        let comment_create = CommentCreate {
            article_id,
            author_id,
            raw_content,
            content,
            status: 0
        };

        match comment_create.insert() {
            Ok(comment) => {
                res_redirect!(format!("/article?id={}", article_id))
            },
            Err(_) => {
                res_500!("comment create error.")
            }
        }  
    }

    pub fn comment_edit(req: &mut Request) -> SapperResult<Response> {
        let params = get_form_params!(req);
        let id = t_param_parse!(params, "id", Uuid);
        let article_id = t_param_parse!(params, "article_id", Uuid);
        let raw_content = t_param!(params, "raw_content");

        let content = markdown_render(raw_content);
        let raw_content = raw_content.to_owned();

        let comment_edit = CommentEdit {
            id,
            raw_content,
            content
        };

        match comment_edit.update() {
            Ok(comment) => {
                res_redirect!(format!("/article?id={}", article_id))
            },
            Err(_) => {
                res_500!("comment edit error.")
            }
        } 
    }

    pub fn comment_delete(req: &mut Request) -> SapperResult<Response> {
        let params = get_form_params!(req);
        let article_id = t_param_parse!(params, "article_id", Uuid);
        let comment_id = t_param_parse!(params, "comment_id", Uuid);

        match Comment::delete_by_id(comment_id) {
            Ok(comment) => {
                res_redirect!(format!("/article?id={}", article_id))
            },
            Err(_) => {
                res_500!("comment delete error.")
            }
        } 
    }


}


impl SapperModule for CommentPage {
    fn before(&self, req: &mut Request) -> SapperResult<()> {
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
            if &path == "/s/comment/new"
                || &path == "/s/comment/edit"
                || &path == "/s/comment/delete" {
                
                let params = get_form_params!(req);
                let article_id = t_param_parse!(params, "article_id", Uuid);

                let ncpp = envconfig::get_int_item("NUMBER_COMMENT_PER_PAGE");
                let n = Article::get_comments_count_belong_to_this(article_id);
                let total_page = ((n -1) / ncpp) as i64 + 1;

                for i in 1..=total_page {
                    let part_key = article_id.to_string() + ":" + &i.to_string();
                    cache::cache_set_invalid("article", &part_key);
                }
                
            }
        }

        Ok(())
    }

    fn router(&self, router: &mut SapperRouter) -> SapperResult<()> {
        router.get("/p/comment/new", Self::comment_new_page);
        router.get("/p/comment/edit", Self::comment_edit_page);
        router.get("/p/comment/delete", Self::comment_delete_page);
        router.post("/s/comment/new", Self::comment_new);
        router.post("/s/comment/edit", Self::comment_edit);
        router.post("/s/comment/delete", Self::comment_delete);
        
        Ok(())
    }
}


