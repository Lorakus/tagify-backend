use crate::models::Roles;

pub fn validate_role(role: &str) -> bool {
    let mut is_role = false;
    for curr in Roles.iter() {
        if **curr == user.role {
            is_role = true;
        }
    }
    is_role
}
