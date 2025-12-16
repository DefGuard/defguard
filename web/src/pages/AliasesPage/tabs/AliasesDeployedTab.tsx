import type { AclAlias } from '../../../shared/api/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';

type Props = {
  aliases: AclAlias[];
};

export const AliasesDeployedTab = ({ aliases }: Props) => {
  const isEmpty = aliases.length === 0;
  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="aliases"
          title={`You haven't created any aliases yet.`}
          subtitle="Click the first alias by clicking button below."
        />
      )}
    </>
  );
};
