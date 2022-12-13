import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { UserProfile } from './UserProfile/UserProfile';
import { UsersSharedModals } from './UsersSharedModals';

/***
 * Only for /me route
 ***/
export const UserProfilePage = () => {
  return (
    <PageContainer id="user-page">
      <UserProfile />
      <UsersSharedModals />
    </PageContainer>
  );
};
