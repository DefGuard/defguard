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
            },
            DirectoryUser {
                email: "testuserdisabled@email.com".into(),
                active: false,
                id: Some("testuserdisabled-id".into()),
            },
            DirectoryUser {
                email: "testuser2@email.com".into(),
                active: true,
                id: Some("testuser2-id".into()),
            },
        ])
    }

    async fn test_connection(&self) -> Result<(), DirectorySyncError> {
        Ok(())
    }
}
