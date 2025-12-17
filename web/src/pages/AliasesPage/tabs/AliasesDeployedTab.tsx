import { useNavigate } from '@tanstack/react-router';
import { useMemo } from 'react';
import type { AclAlias } from '../../../shared/api/types';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { ButtonProps } from '../../../shared/defguard-ui/components/Button/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';
import { TableTop } from '../../../shared/defguard-ui/components/table/TableTop/TableTop';
import { AliasTable } from '../AliasTable';

type Props = {
  aliases: AclAlias[];
};

export const AliasesDeployedTab = ({ aliases }: Props) => {
  const isEmpty = aliases.length === 0;
  const navigate = useNavigate();

  const addButtonProps = useMemo(
    (): ButtonProps => ({
      text: 'Add new alias',
      iconLeft: 'add-alias',
      variant: 'primary',
      onClick: () => {
        navigate({ to: '/acl/add-alias' });
      },
    }),
    [navigate],
  );

  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="aliases"
          title={`You haven't created any aliases yet.`}
          subtitle="Click the first alias by clicking button below."
          primaryAction={addButtonProps}
        />
      )}
      {!isEmpty && (
        <>
          <TableTop text="Deployed aliases">
            <Button {...addButtonProps} />
          </TableTop>
          <AliasTable data={aliases} />
        </>
      )}
    </>
  );
};
