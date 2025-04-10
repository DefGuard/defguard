import { useMutation } from '@tanstack/react-query';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import useApi from '../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../shared/hooks/useToaster';

export const LdapConnectionTest = () => {
  const { LL } = useI18nContext();
  const localLL = LL.settingsPage.ldapSettings.test;
  const {
    settings: { testLdapSettings },
  } = useApi({ notifyError: false });

  const toaster = useToaster();

  const { isPending: isLoading, mutate } = useMutation({
    mutationFn: testLdapSettings,
    onSuccess: () => {
      toaster.success(localLL.messages.success());
    },
    onError: () => {
      toaster.error(localLL.messages.error());
    },
  });

  return (
    <Button
      size={ButtonSize.SMALL}
      styleVariant={ButtonStyleVariant.LINK}
      // text={localLL.submit()}
      text="Test LDAP connection"
      // icon={<SvgIconCheckmark />}
      loading={isLoading}
      onClick={() => {
        mutate();
      }}
    />
  );
};
