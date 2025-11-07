import { useSuspenseQuery } from '@tanstack/react-query';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { getGroupsInfoQueryOptions, getUsersQueryOptions } from '../../shared/query';
import { GroupsTable } from './components/GroupsTable/GroupsTable';
import { AddGroupModal } from './modals/AddGroupModal/AddGroupModal';

export const GroupsPage = () => {
  const { data: groups } = useSuspenseQuery(getGroupsInfoQueryOptions);
  const { data: users } = useSuspenseQuery(getUsersQueryOptions);
  return (
    <>
      <Page id="groups-page" title="Groups">
        <SizedBox height={ThemeSpacing.Xl3} />
        <GroupsTable groups={groups} users={users} />
      </Page>
      <AddGroupModal />
    </>
  );
};
