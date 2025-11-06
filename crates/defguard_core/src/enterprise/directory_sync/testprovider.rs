use super::{DirectoryGroup, DirectorySync, DirectorySyncError, DirectoryUser};

#[allow(dead_code)]
pub(crate) struct TestProviderDirectorySync;

impl DirectorySync for TestProviderDirectorySync {
    async fn get_groups(&self) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        Ok(vec![
            DirectoryGroup {
                id: "1".into(),
                name: "group1".into(),
            },
            DirectoryGroup {
                id: "2".into(),
                name: "group2".into(),
            },
            DirectoryGroup {
                id: "3".into(),
                name: "group3".into(),
            },
        ])
    }

    async fn get_user_groups(
        &self,
        _user_email: &str,
    ) -> Result<Vec<DirectoryGroup>, DirectorySyncError> {
        Ok(vec![DirectoryGroup {
            id: "1".into(),
            name: "group1".into(),
        }])
    }

    async fn get_group_members(
        &self,
        _group: &DirectoryGroup,
        _all_users_helper: Option<&[DirectoryUser]>,
    ) -> Result<Vec<String>, DirectorySyncError> {
        Ok(vec![
            "testuser@email.com".into(),
            "testuserdisabled@email.com".into(),
            "testuser2@email.com".into(),
        ])
    }

    async fn prepare(&mut self) -> Result<(), DirectorySyncError> {
        Ok(())
    }

    async fn get_all_users(&self) -> Result<Vec<DirectoryUser>, DirectorySyncError> {
        Ok(vec![
            DirectoryUser {
                email: "testuser@email.com".into(),
                active: true,
                id: Some("testuser-id".into()),
                user_details: Some(crate::enterprise::directory_sync::DirectoryUserDetails {
                    username: "testuser".into(),
                    last_name: "User".into(),
                    first_name: "Test".into(),
                    phone_number: None,
                    openid_sub: "testuser-id".into(),
                }),
            },
            DirectoryUser {
                email: "testuserdisabled@email.com".into(),
                active: false,
                id: Some("testuserdisabled-id".into()),
                user_details: Some(crate::enterprise::directory_sync::DirectoryUserDetails {
                    username: "testuserdisabled".into(),
                    last_name: "UserDisabled".into(),
                    first_name: "Test".into(),
                    phone_number: None,
                    openid_sub: "testuserdisabled-id".into(),
                }),
            },
            DirectoryUser {
                email: "testuser2@email.com".into(),
                active: true,
                id: Some("testuser2-id".into()),
                user_details: Some(crate::enterprise::directory_sync::DirectoryUserDetails {
                    username: "testuser2".into(),
                    last_name: "User2".into(),
                    first_name: "Test".into(),
                    phone_number: None,
                    openid_sub: "testuser2-id".into(),
                }),
            },
        ])
    }

    async fn test_connection(&self) -> Result<(), DirectorySyncError> {
        Ok(())
    }
}
