use sapper::Request;
use sapper_std::*;
use crate::AppUser;

pub fn permission_need_login(req: &mut Request) -> Result<bool, String> {
    let (path, _) = req.uri();
    if path.starts_with("/s/") || path.starts_with("/p/")
    {
        match ext_type!(req, AppUser) {
            Some(ref _user) => {
                // pass, nothing need to do here
                return Ok(true);
            },
            None => {
                return Err("No permissions: need login.".to_string());
            }
        }
    }
    else {
        Ok(true)
    }
}

pub fn permission_need_be_admin(req: &mut Request) -> Result<bool, String> {
    let (path, _) = req.uri();
    if path.starts_with("/s/") || path.starts_with("/p/")
    {
        match ext_type!(req, AppUser) {
            Some(user) => {
                if user.role >= 9 {
                    // pass, nothing need to do here
                    return Ok(true);

                }
                else {
                    return Err("No permissions: need be admin.".to_string());
                }
            },
            None => {
                return Err("No permissions: need login.".to_string());
            }
        }
    }
    else {
        Ok(true)
    }
}

