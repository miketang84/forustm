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

use crate::dataservice::section::{
    Section,
    SectionNew,
    SectionEdit,
    UpdateSectionWeight
};

use crate::middleware::{
    permission_need_be_admin,
    permission_need_login,
    is_admin,
};
use crate::envconfig;


pub struct SectionPage;

impl SectionPage {

    pub fn section_create_page(req: &mut Request) -> SapperResult<Response> {
        permission_need_be_admin(req)?;
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();

        res_html!("forum/new_section.html", web)
    }

    pub fn section_edit_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        
        let (path, _) = req.uri();
        if path == "/p/blogsection/edit" {
            let user = ext_type!(req, AppUser).unwrap();
            match Section::get_by_suser(user.id) {
                Ok(section) => {
                    web.insert("section", &section);
                    res_html!("forum/edit_section.html", web)
                },
                Err(info) => {
                    res_400!(info)
                }
            }
        }
        else {
            let params = get_query_params!(req);
            let section_id = t_param_parse!(params, "id", Uuid);

            if is_admin(req) {
                let section = Section::get_by_id(section_id).unwrap();
                web.insert("section", &section);
                res_html!("forum/edit_section.html", web)
            }
            else {
                let user = ext_type!(req, AppUser).unwrap();
                match Section::get_by_suser(user.id) {
                    Ok(section) => {
                        if section.id == section_id {
                            web.insert("section", &section);
                            res_html!("forum/edit_section.html", web)
                        }
                        else {
                            res_400!("no permission.".to_string())
                        }
                    },
                    Err(info) => {
                        res_400!(info)
                    }
                }
            }
        }
    }
    
    pub fn section_detail_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);
        
        let (path, _) = req.uri();
        let section_id = if path == "/blog_with_author" {
            let author_id = t_param_parse!(params, "author_id", Uuid);
            let section = Section::get_by_suser(author_id);
            if section.is_err() {
                return res_400!("no this section");
            }
            let section = section.unwrap();
            section.id
        }
        else {
            t_param_parse!(params, "id", Uuid)
        };

        let current_page = t_param_parse_default!(params, "current_page", i64, 1);

        let section_result = Section::get_by_id(section_id);
        if section_result.is_err() {
            return res_400!("no this section");
        }
        
        let section = section_result.unwrap();
        let mut is_a_blog = false;
        if section.stype == 1 {
            is_a_blog = true;
        }
        let mut is_myown_blog = false;
        let mut is_admin = false;
        let mut is_login = false;
        match ext_type!(req, AppUser) {
            Some(user) => {
                if section.suser == Some(user.id) {
                    is_myown_blog = true;
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

        let napp = envconfig::get_int_item("NUMBER_ARTICLE_PER_PAGE");
        let total_item = Section::get_articles_count_belong_to_this(section.id);
        let total_page = ((total_item - 1) / napp) as i64 + 1;

        let articles = Section::get_articles_paging_belong_to_this(section.id, current_page);

        web.insert("section", &section);
        web.insert("is_a_blog", &is_a_blog);
        web.insert("is_myown_blog", &is_myown_blog);
        web.insert("is_admin", &is_admin);
        web.insert("total_item", &total_item);
        web.insert("total_page", &total_page);
        web.insert("current_page", &current_page);
        web.insert("articles", &articles);

        res_html!("forum/section.html", web)
    }



    pub fn section_create(req: &mut Request) -> SapperResult<Response> {
        permission_need_be_admin(req)?;
        let params = get_form_params!(req);
        let title = t_param!(params, "title").to_owned();
        let description = t_param!(params, "description").to_owned();

        let section_new = SectionNew {
            title,
            description
        };

        match section_new.create() {
            Ok(section) => {
                res_redirect!(format!("/section?id={}", section.id))
            },
            Err(_) => {
                res_500!("section create error.")
            }
        }  
    }

    pub fn section_edit(req: &mut Request) -> SapperResult<Response> {
        let params = get_form_params!(req);
        let id = t_param_parse!(params, "id", Uuid);
        let title = t_param!(params, "title").to_owned();
        let description = t_param!(params, "description").to_owned();

        let section_edit = SectionEdit {
            id,
            title,
            description
        };

        match section_edit.update() {
            Ok(section) => {
                res_redirect!(format!("/section?id={}", section.id))
            },
            Err(_) => {
                res_500!("section edit error.")
            }
        }  
    }

    pub fn section_rearrange_page(req: &mut Request) -> SapperResult<Response> {
        permission_need_be_admin(req)?;
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();

        let sections = Section::forum_sections();

        web.insert("sections", &sections);

        res_html!("forum/arrange_sections.html", web)
    }


    pub fn section_rearrange(req: &mut Request) -> SapperResult<Response> {
        permission_need_be_admin(req)?;
        let mut web = ext_type_owned!(req, AppWebContext).unwrap();
        let params = get_form_params!(req);
        let order = t_arr_param!(params, "order");

        // print order
        let sections = Section::forum_sections();
        for (i, section) in sections.iter().enumerate() {
            let update_section_weight = UpdateSectionWeight {
                id: section.id,
                weight: order[i].parse::<f64>().unwrap()
            };
            update_section_weight.update().unwrap();
        }
        
        res_redirect!("/p/section/rearrange")
    }

}


impl SapperModule for SectionPage {
    fn before(&self, req: &mut Request) -> SapperResult<()> {
        // check cache
        let (path, _) = req.uri();
        if envconfig::get_int_item("CACHE") == 1 {
            if &path == "/section" || &path == "/blog" {
                let params = get_query_params!(req);
                let section_id = t_param!(params, "id");
                let current_page = t_param_parse_default!(params, "current_page", i64, 1);
                let part_key = section_id.to_string() + ":" + &current_page.to_string();
                if cache::cache_is_valid("section", &part_key) {
                    let cache_content = cache::cache_get("section", &part_key);
                    return res_html_before!(cache_content);
                }
            }
        }

        // permission
        permission_need_login(req)?;
        
        Ok(())
    }

    fn after(&self, req: &Request, res: &mut Response) -> SapperResult<()> {
        let res_status = res.status();
        if res_status == status::Ok || res_status == status::Found {
            let (path, _) = req.uri();
            if &path == "/s/section/create" 
                || &path == "/s/section/edit" 
                || &path == "/s/section/rearrange" {
            
                cache::cache_set_invalid("index", "index");
            }

            // check other url
            if &path == "/section" || &path == "/blog" {
                let params = get_query_params!(req);
                let section_id = t_param!(params, "id");
                let current_page = t_param_parse_default!(params, "current_page", i64, 1);
                let part_key = section_id.to_string() + ":" + &current_page.to_string();
                if !cache::cache_is_valid("section", &part_key) {
                    cache::cache_set("section", &part_key, res.body());
                }
            }
            
            if &path == "/s/section/edit" {
                let params = get_form_params!(req);
                let section_id = t_param!(params, "id");
                cache::cache_set_invalid("section", section_id);
            }


        }

        Ok(())
    }

    fn router(&self, router: &mut SapperRouter) -> SapperResult<()> {
        router.get("/section", Self::section_detail_page);
        router.get("/blog", Self::section_detail_page);
        router.get("/blog_with_author", Self::section_detail_page);

        router.get("/p/section/create", Self::section_create_page);
        router.get("/p/section/edit", Self::section_edit_page);
        router.get("/p/blogsection/edit", Self::section_edit_page);
        router.post("/s/section/create", Self::section_create);
        router.post("/s/section/edit", Self::section_edit);

        router.get("/p/section/rearrange", Self::section_rearrange_page);
        router.post("/s/section/rearrange", Self::section_rearrange);


        Ok(())
    }
}


