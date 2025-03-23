use crate::db::User;

impl<I> User<I> {
    pub(crate) fn update_from_ldap_user(&mut self, ldap_user: &User) {
        self.last_name = ldap_user.last_name.clone();
        self.first_name = ldap_user.first_name.clone();
        self.email = ldap_user.email.clone();
        self.phone = ldap_user.phone.clone();
    }
}
