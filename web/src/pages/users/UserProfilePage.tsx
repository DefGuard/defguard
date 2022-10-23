import PageContainer from '../../shared/components/layout/PageContainer/PageContainer';
import { UserProfileV2 } from './UserProfileV2/UserProfileV2';
import { UsersSharedModals } from './UsersSharedModals';

/***
 * Only for /me route
 ***/
export const UserProfilePage = () => {
  return (
    <PageContainer id="user-page">
      <UserProfileV2 />
      <UsersSharedModals />
    </PageContainer>
  );
};
