import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { m } from '../../../paraglide/messages';
import { AclListTab } from '../../../shared/aclTabs';
import api from '../../../shared/api/api';
import { AclStatus } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { getAliasesQueryOptions, getRulesQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature, licenseActionCheck } from '../../../shared/utils/license';
import { useRuleDeps } from '../../RulesPage/useRuleDeps';
import { AliasTable } from '../AliasTable';

export const AliasesPendingTab = () => {
  const { data: aliases } = useSuspenseQuery({
    ...getAliasesQueryOptions,
    select: (resp) => resp.data.filter((alias) => alias.state !== AclStatus.Applied),
  });
  const { license, loading } = useRuleDeps();
  const { data: rules } = useSuspenseQuery(getRulesQueryOptions);
  const isEmpty = aliases.length === 0;
  const { mutate: applyAliases, isPending } = useMutation({
    mutationFn: api.acl.alias.applyAliases,
    meta: {
      invalidate: ['acl'],
    },
  });

  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="aliases"
          title={m.acl_destinations_empty_pending_title()}
          subtitle={m.acl_destinations_empty_pending_subtitle()}
        />
      )}
      {!isEmpty && (
        <>
          <TableTop text={m.acl_aliases_table_title_pending()}>
            {aliases.length > 0 && (
              <Button
                variant="primary"
                iconLeft="deploy"
                text={m.acl_destinations_button_deploy_all_pending({
                  count: aliases.length,
                })}
                loading={isPending}
                disabled={loading}
                onClick={() => {
                  if (license === undefined) return;
                  licenseActionCheck(canUseBusinessFeature(license), () => {
                    applyAliases(aliases.map((alias) => alias.id));
                  });
                }}
              />
            )}
          </TableTop>
          <AliasTable
            data={aliases}
            rules={rules}
            tab={AclListTab.Pending}
            disableBlockedModal
          />
        </>
      )}
    </>
  );
};
