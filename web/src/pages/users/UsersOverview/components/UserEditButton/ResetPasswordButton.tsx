import { useMutation } from '@tanstack/react-query';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
import { useAppStore } from '../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';
import { MutationKeys } from '../../../../../shared/mutations';
import { User } from '../../../../../shared/types';

type Props = {
  user: User;
};

export const ResetPasswordButton = ({ user }: Props) => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const appInfo = useAppStore((s) => s.appInfo);

  const {
    user: { resetPassword },
  } = useApi();

  const changePasswordMutation = useMutation(resetPassword, {
    mutationKey: [MutationKeys.RESET_PASSWORD],
    onSuccess: () => {
      toaster.success(LL.userPage.messages.passwordResetEmailSent());
    },
    onError: (e) => {
      toaster.error(LL.messages.error());
      console.error(e);
    },
  });

  return (
    <EditButtonOption
      key="reset-password"
      text={LL.usersOverview.list.editButton.resetPassword()}
      onClick={() => {
        changePasswordMutation.mutate({ username: user.username });
      }}
      // disable if smtp is not configured
      disabled={!appInfo || (appInfo && !appInfo.smtp_enabled)}
    />
  );
};
