import type { AclRule } from '../../../shared/api/types';
import { EmptyStateFlexible } from '../../../shared/defguard-ui/components/EmptyStateFlexible/EmptyStateFlexible';

type Props = {
  rules: AclRule[];
};

export const RulesPendingTab = ({ rules }: Props) => {
  const isEmpty = rules.length === 0;
  return (
    <>
      {isEmpty && (
        <EmptyStateFlexible
          icon="rules"
          title={`You don't have any pending rules.`}
          subtitle={`They will appear here once your create your first rule.`}
        />
      )}
    </>
  );
};
