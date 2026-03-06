import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import api from '../../../shared/api/api';
import { AclStatus } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { getAliasesQueryOptions, getRulesQueryOptions } from '../../../shared/query';
import { AliasTable } from '../AliasTable';

export const AliasesPendingTab = () => {
  const { data: aliases } = useSuspenseQuery({
    ...getAliasesQueryOptions,
    select: (resp) => resp.data.filter((alias) => alias.state !== AclStatus.Applied),
  });
  const { data: rules } = useSuspenseQuery(getRulesQueryOptions);
  const isEmpty = aliases.length === 0;
  const { mutate: applyAliases, isPending } = useMutation({
    mutationFn: api.acl.alias.applyAliases,
    meta: {
      invalidate: ['acl', 'alias'],
    },
  });

  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="aliases"
          title={`You don't have any pending items.`}
          subtitle={`They will appear here once you add or modify one.`}
        />
      )}
      {!isEmpty && (
        <>
          <TableTop text="Pending aliases">
            {aliases.length > 0 && (
              <Button
                variant="primary"
                iconLeft="deploy"
                text={`Deploy all pending (${aliases.length})`}
                loading={isPending}
                onClick={() => {
                  applyAliases(aliases.map((alias) => alias.id));
                }}
              />
            )}
          </TableTop>
          <AliasTable data={aliases} rules={rules} disableBlockedModal />
        </>
      )}
    </>
  );
};
