use sapper::{
    Request,
    Response,
    Result as SapperResult,
    Error as SapperError,
    Module as SapperModule,
    Router as SapperRouter};
use sapper_std::*;
use uuid::Uuid;

use crate::db;
use crate::github_utils::{
    get_github_token,
    get_github_user_info,
};

use crate::envconfig;
use crate::util::make_pwd_encode;
use crate::util::random_string;
use crate::middleware::permission_need_login;

// introduce macros
use crate::{
    AppWebContext,
    AppUser
};

use crate::dataservice::user::{
    Ruser,
    UserLogin,
    UserSignUp,
    GithubUserInfo,
    UpdateUserNickname,
    UserChangePassword,
};

use crate::dataservice::article::{
    Article
};

pub struct UserPage;

impl UserPage {

    pub fn page_login_with3rd(req: &mut Request) -> SapperResult<Response> {
        let mut web = get_ext_owned!(req, AppWebContext).unwrap();

        let client_id = envconfig::get_str_item("GITHUB_APP_CLIENT_ID");
        web.insert("client_id", &client_id);

        res_html!("forum/login_with3rd.html", web)
    }

    pub fn page_login_with_admin(req: &mut Request) -> SapperResult<Response> {
        let web = get_ext_owned!(req, AppWebContext).unwrap();

        res_html!("forum/login_with_admin.html", web)
    }

    pub fn account(req: &mut Request) -> SapperResult<Response> {
        let mut web = get_ext_owned!(req, AppWebContext).unwrap();
        match get_ext!(req, AppUser) {
            Some(user) => {
                web.insert("user", &user);
                return res_html!("forum/account.html", web);
            },
            None => {
                let client_id = envconfig::get_str_item("GITHUB_APP_CLIENT_ID");
                web.insert("client_id", &client_id);

                return res_html!("forum/login_with3rd.html", web);
            }
        }
    }

    pub fn user_register(req: &mut Request) -> SapperResult<Response> {

        let params = get_form_params!(req);
        let account = t_param!(params, "account").to_owned();
        let password = t_param!(params, "password").to_owned();
        let nickname = t_param!(params, "nickname").to_owned();

        let user_signup = UserSignUp {
            account,
            password,
            nickname
        };

        // TODO: need to check the result of this call
        let _ = user_signup.sign_up(None);

        // redirect to login with account and password
        res_redirect!("/login_with_admin")
    }

    pub fn user_login(req: &mut Request) -> SapperResult<Response> {

        let params = get_form_params!(req);
        let account = t_param!(params, "account").to_owned();
        let password = t_param!(params, "password").to_owned();

        let user_login = UserLogin {
            account,
            password
        };

        // use dataservice logic
        let cookie_r = user_login.verify_login();
        if cookie_r.is_err() {
            return res_redirect!("/login_with_admin");
        }
        let cookie = cookie_r.unwrap();

        let mut response = Response::new();
        let _ = set_cookie(
            &mut response,
            "rusoda_session".to_string(),
            cookie,
            None,
            Some("/".to_string()),
            None,
            Some(60*24*3600),
        );

        // redirect to index
        set_response_redirect!(response, "/");

        Ok(response)
    }

    pub fn user_login_with_github(req: &mut Request) -> SapperResult<Response> {

        let params = get_query_params!(req);
        let code = t_param!(params, "code");

        let client_id = envconfig::get_str_item("GITHUB_APP_CLIENT_ID");
        let client_secret = envconfig::get_str_item("GITHUB_APP_CLIENT_SECRET");

        let token_r = get_github_token(&code, client_id, client_secret);
        if token_r.is_err() {
            return res_400!("get github token code err");
        }
        let access_token = token_r.unwrap();
        let github_user_info: GithubUserInfo = get_github_user_info(&access_token).unwrap();

        let account = github_user_info.account;
        let password;
        let cookie;


        match Ruser::get_user_by_account(&account) {
            Ok(user) => {
                // already exists
                password = user.password;
                // next step auto login
                let user_login = UserLogin {
                        account,
                        password
                };
                cookie = user_login.verify_login_with_rawpwd().unwrap();
            },
            Err(_) => {
                password = random_string(8);
                // register it
                let user_signup = UserSignUp {
                    account: account.clone(),
                    password: password.clone(),
                    nickname: account.clone(),
                };
                // TODO: check the result
                let _ = user_signup.sign_up(Some(github_user_info.github_address));
                // next step auto login
                let user_login = UserLogin {
                        account,
                        password
                };
                cookie = user_login.verify_login().unwrap();
            }
        }

        let mut response = Response::new();
        let _ = set_cookie(
            &mut response,
            "rusoda_session".to_string(),
            cookie,
            None,
            Some("/".to_string()),
            None,
            Some(60*24*3600),
        );

        // redirect to index
        set_response_redirect!(response, "/");

        Ok(response)
    }


    pub fn user_signout(req: &mut Request) -> SapperResult<Response> {
        match get_ext!(req, SessionVal) {
            Some(cookie) => {
                let _ = Ruser::sign_out(cookie);
            },
            None => {}
        }

        res_redirect!("/")
    }

    pub fn user_modifynickname_page(req: &mut Request) -> SapperResult<Response> {
        let web = get_ext_owned!(req, AppWebContext).unwrap();

        res_html!("forum/user_modifynickname_page.html", web)
    }

    pub fn user_modifynickname(req: &mut Request) -> SapperResult<Response> {
        let web = get_ext_owned!(req, AppWebContext).unwrap();
        let params = get_form_params!(req);
        let nickname = t_param!(params, "nickname").to_owned();

        let user = get_ext!(req, AppUser).unwrap();
        let id = user.id;

        let update_user_nickname = UpdateUserNickname {
            id,
            nickname
        };

        update_user_nickname.update().unwrap();

        res_redirect!("/account")
    }

    pub fn user_changepassword_page(req: &mut Request) -> SapperResult<Response> {
        let web = get_ext_owned!(req, AppWebContext).unwrap();

        res_html!("forum/user_change_pwd_page.html", web)
    }

    pub fn user_changepassword(req: &mut Request) -> SapperResult<Response> {
        let web = get_ext_owned!(req, AppWebContext).unwrap();
        let params = get_form_params!(req);
        let old_pwd = t_param!(params, "old_pwd");
        let new_pwd = t_param!(params, "new_pwd");

        let user = get_ext!(req, AppUser).unwrap();
        let id = user.id;

        // check password equality
        if user.password == make_pwd_encode(old_pwd, &user.salt) {
            let user_change_pwd = UserChangePassword {
                id,
                password: make_pwd_encode(new_pwd, &user.salt)
            };

            let _ = user_change_pwd.change();

            res_redirect!("/account")
        }
        else {
            res_400!("not corrent old password.")
        }
    }

    pub fn user_my_articles_page(req: &mut Request) -> SapperResult<Response> {
        let mut web = get_ext_owned!(req, AppWebContext).unwrap();
        let params = get_query_params!(req);

        let current_page = t_param_parse_default!(params, "current_page", i64, 1);

        let mut is_admin = false;
        let mut is_login = false;
        let mut user_id: Uuid = Default::default();
        match get_ext!(req, AppUser) {
            Some(user) => {
                user_id = user.id;

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
        let total_item = Article::get_all_articles_count_by_author(user_id);
        let total_page = ((total_item - 1) / napp) as i64 + 1;

        let articles = Article::get_latest_articles_paging_by_author(user_id, current_page-1, napp);

        web.insert("is_admin", &is_admin);
        web.insert("total_item", &total_item);
        web.insert("total_page", &total_page);
        web.insert("current_page", &current_page);
        web.insert("articles", &articles);
        web.insert("this_page_url", "p/user/my_articles");
        web.insert("s_title", "User's Articles");

        res_html!("forum/article_list_paging.html", web)
    }


}


impl SapperModule for UserPage {
    fn before(&self, req: &mut Request) -> SapperResult<()> {
        permission_need_login(req)?;

        Ok(())
    }

    fn router(&self, router: &mut SapperRouter) -> SapperResult<()> {
        router.get("/login_with3rd", Self::page_login_with3rd);
        router.get("/login_with_admin", Self::page_login_with_admin);
        router.get("/account", Self::account);
        router.get("/signout", Self::user_signout);


        router.post("/register", Self::user_register);
        router.post("/login", Self::user_login);

        router.get("/p/user/modifynickname", Self::user_modifynickname_page);
        router.post("/s/user/modifynickname", Self::user_modifynickname);

        router.get("/p/user/changepassword", Self::user_changepassword_page);
        router.post("/s/user/changepassword", Self::user_changepassword);

        router.get("/p/user/my_articles", Self::user_my_articles_page);


        // this url will be called by remote github oauth2 server
        router.get("/api/v1/login_with_github", Self::user_login_with_github);


        Ok(())
    }
}
