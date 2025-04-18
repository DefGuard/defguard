import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';

type Props = {
  isOpen: boolean;
  setOpen: (val: boolean) => void;
  onSubmit: () => void;
  changesCount: number;
};

export const AclRulesApplyConfirmModal = ({
  isOpen,
  setOpen,
  onSubmit,
  changesCount,
}: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.acl.listPage.rules.modals.applyConfirm;
  const close = () => setOpen(false);

  return (
    <ConfirmModal
      title={localLL.title()}
      subTitle={localLL.subtitle({
        count: changesCount,
      })}
      submitText={localLL.submit()}
      isOpen={isOpen}
      onClose={() => {
        close();
      }}
      onSubmit={() => {
        onSubmit();
        close();
      }}
    />
  );
};
