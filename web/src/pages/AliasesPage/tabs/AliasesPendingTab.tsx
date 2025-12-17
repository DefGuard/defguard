import type { AclAlias } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { AliasTable } from '../AliasTable';

type Props = {
  aliases: AclAlias[];
};

export const AliasesPendingTab = ({ aliases }: Props) => {
  const isEmpty = aliases.length === 0;
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
            <Button
              variant="primary"
              iconLeft="deploy"
              text={`Deploy all pending (${aliases.length})`}
              onClick={() => {}}
              disabled
            />
          </TableTop>
          <AliasTable data={aliases} />
        </>
      )}
    </>
  );
};
