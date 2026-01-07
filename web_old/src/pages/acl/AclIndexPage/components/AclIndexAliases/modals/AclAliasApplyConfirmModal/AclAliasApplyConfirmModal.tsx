import './style.scss';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';

type Props = {
  isOpen: boolean;
  setOpen: (val: boolean) => void;
  rules: string[];
  onSubmit: () => void;
};

export const AclAliasApplyConfirmModal = ({
  isOpen,
  onSubmit,
  rules,
  setOpen,
}: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.aliases.modals.applyConfirm;

  return (
    <ConfirmModal
      id="acl-aliases-apply-confirm-modal"
      isOpen={isOpen}
      onClose={() => {
        setOpen(false);
      }}
      onSubmit={() => {
        onSubmit();
      }}
      submitText={localLL.submit()}
      title={localLL.title()}
    >
      <div className="content">
        <p>{localLL.message()}</p>
        <p>{`${localLL.listLabel()}(${rules.length})`}:</p>
        <p className="rules">{rules.join(', ')}</p>
      </div>
    </ConfirmModal>
  );
};
