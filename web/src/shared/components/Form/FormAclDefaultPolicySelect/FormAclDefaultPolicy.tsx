import { useMemo } from 'react';
import { FieldValues, UseControllerProps } from 'react-hook-form';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormSelect } from '../../../defguard-ui/components/Form/FormSelect/FormSelect';
import { SelectOption } from '../../../defguard-ui/components/Layout/Select/types';
import { useAppStore } from '../../../hooks/store/useAppStore';

type Props<T extends FieldValues> = {
  controller: UseControllerProps<T>;
};

export const FormAclDefaultPolicy = <T extends FieldValues>({ controller }: Props<T>) => {
  const enterpriseEnabled = useAppStore((s) => s.appInfo?.license_info.enterprise);
  const { LL } = useI18nContext();

  const options = useMemo(
    (): SelectOption<boolean>[] => [
      {
        key: 'allow',
        value: true,
        label: LL.components.aclDefaultPolicySelect.options.allow(),
      },
      {
        key: 'deny',
        value: true,
        label: LL.components.aclDefaultPolicySelect.options.deny(),
      },
    ],
    [LL.components.aclDefaultPolicySelect.options],
  );
  return (
    <FormSelect
      controller={controller}
      options={options}
      placeholder={LL.components.aclDefaultPolicySelect.label()}
      disabled={!enterpriseEnabled}
    />
  );
};
