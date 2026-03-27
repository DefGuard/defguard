import { useSuspenseQuery } from '@tanstack/react-query';
import { m } from '../../paraglide/messages';
import { Page } from '../../shared/components/Page/Page';
import { SizedBox } from '../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../shared/defguard-ui/types';
import { TablePageLayout } from '../../shared/layout/TablePageLayout/TablePageLayout';
import {
  getGroupsInfoQueryOptions,
  getUsersOverviewQueryOptions,
} from '../../shared/query';
import { GroupsTable } from './components/GroupsTable/GroupsTable';
import { CEGroupModal } from './modals/CEGroupModal/CEGroupModal';

export const GroupsPage = () => {
  const { data: groups } = useSuspenseQuery(getGroupsInfoQueryOptions);
  const { data: users } = useSuspenseQuery(getUsersOverviewQueryOptions);
  return (
    <>
      <Page id="groups-page" title={m.groups_title()}>
        <SizedBox height={ThemeSpacing.Xl3} />
        <TablePageLayout>
          <GroupsTable groups={groups} users={users} />
        </TablePageLayout>
      </Page>
      <CEGroupModal />
    </>
  );
};
