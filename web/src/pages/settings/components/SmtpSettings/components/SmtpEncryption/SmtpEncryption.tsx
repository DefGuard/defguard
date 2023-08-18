import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useCallback, useMemo } from 'react';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { Select } from '../../../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectOption,
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Select/types';
import { useAppStore } from '../../../../../../shared/hooks/store/useAppStore';
import useApi from '../../../../../../shared/hooks/useApi';
import { useToaster } from '../../../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../../../shared/queries';

export const SmtpEncryption = () => {
  const {
    settings: { editSettings },
  } = useApi();
  const { LL } = useI18nContext();

  const settings = useAppStore((state) => state.settings);

  const queryClient = useQueryClient();

  const toaster = useToaster();

  const encryptionOptions = useMemo(
    (): SelectOption<string>[] => [
      {
        key: 1,
        value: 'StartTls',
        label: 'Start TLS',
      },
      {
        key: 2,
        value: 'None',
        label: 'None',
      },
      {
        key: 3,
        value: 'ImplicitTls',
        label: 'Implicit TLS',
      },
    ],
    [],
  );

  const renderSelectedEncryption = useCallback(
    (selected: string): SelectSelectedValue => {
      const option = encryptionOptions.find((o) => o.value === selected);
      if (!option) throw Error("Selected value doesn't exist");
      return {
        key: option.key,
        displayValue: option.label,
      };
    },
    [encryptionOptions],
  );

  const { isLoading, mutate } = useMutation({
    mutationFn: editSettings,
    onSuccess: () => {
      toaster.success(LL.settingsPage.messages.editSuccess());
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
    },
    onError: (e) => {
      toaster.error(LL.messages.error());
      console.error(e);
    },
  });

  return (
    <section id="smtp-encryption">
      <header>
        <h2>{LL.settingsPage.smtp.encryption.title()}</h2>
      </header>
      <Select
        sizeVariant={SelectSizeVariant.SMALL}
        data-testid="smtp-encryption-select"
        renderSelected={renderSelectedEncryption}
        options={encryptionOptions}
        label={LL.settingsPage.smtp.form.fields.encryption.label()}
        selected={settings?.smtp_encryption}
        disabled={isUndefined(settings)}
        loading={isLoading}
        onChangeSingle={(res) => {
          if (!isLoading && settings) {
            mutate({ ...settings, smtp_encryption: res });
          }
        }}
      />
    </section>
  );
};
