import { useMutation } from '@tanstack/react-query';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { EditButtonOption } from '../../../../../shared/defguard-ui/components/Layout/EditButton/EditButtonOption';
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

  const {
    user: { resetPassword },
  } = useApi();

  const changePasswordMutation = useMutation(resetPassword, {
    mutationKey: [MutationKeys.RESET_PASSWORD],
    onSuccess: () => {
      toaster.success(LL.userPage.messages.passwordResetEmailSent());
    },
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    onError: (_err) => {
      toaster.error(LL.messages.error());
    },
  });

  return (
    <EditButtonOption
      key="reset-password"
      text={LL.usersOverview.list.editButton.resetPassword()}
      onClick={() => {
        changePasswordMutation.mutate({ username: user.username });
      }}
    />
  );
};
