import './style.scss';

import { PageContainer } from '../../shared/components/Layout/PageContainer/PageContainer';
import { GroupsManagement } from './components/GroupsManagement/GroupsManagement';
import { AddGroupModal } from './components/modals/AddGroupModal/AddGroupModal';

export const GroupsPage = () => {
  return (
    <PageContainer id="groups-page">
      <GroupsManagement />
      <AddGroupModal />
    </PageContainer>
  );
};
