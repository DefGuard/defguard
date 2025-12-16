import type { AclAlias } from '../../../shared/api/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';

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
    </>
  );
};
