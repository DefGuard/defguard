import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { RenderMarkdown } from '../../../../../../../shared/components/Layout/RenderMarkdown/RenderMarkdown';
import { ConfirmModal } from '../../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import { isPresent } from '../../../../../../../shared/defguard-ui/utils/isPresent';
import { useAclAliasDeleteBlockModal } from './store';

export const AclAliasDeleteBlockModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.aliases.modals.deleteBlock;
  const [close, reset] = useAclAliasDeleteBlockModal((s) => [s.close, s.reset], shallow);
  const alias = useAclAliasDeleteBlockModal((s) => s.alias);
  const isOpen = useAclAliasDeleteBlockModal((s) => s.visible);

  useEffect(() => {
    return () => {
      reset?.();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <ConfirmModal
      type={ConfirmModalType.WARNING}
      title={localLL.title()}
      isOpen={isOpen}
      onClose={() => {
        close();
      }}
      afterClose={() => {
        reset();
      }}
    >
      {isPresent(alias) && (
        <RenderMarkdown
          content={localLL.content({
            rulesCount: alias.rules.length,
          })}
        />
      )}
    </ConfirmModal>
  );
};
