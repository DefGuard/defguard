import { useMemo } from 'react';
import type { FieldValues, UseControllerProps } from 'react-hook-form';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormSelect } from '../../../defguard-ui/components/Form/FormSelect/FormSelect';
import type { SelectOption } from '../../../defguard-ui/components/Layout/Select/types';
import { LocationMfaType } from '../../../types';

type Props<T extends FieldValues> = {
  controller: UseControllerProps<T>;
  disabled?: boolean;
};

export const FormLocationMfaTypeSelect = <T extends FieldValues>({
  controller,
  disabled = false,
}: Props<T>) => {
  const { LL } = useI18nContext();

  const options = useMemo(
    (): SelectOption<LocationMfaType>[] => [
      {
        key: LocationMfaType.DISABLED,
        value: LocationMfaType.DISABLED,
        label: LL.components.locationMfaTypeSelect.options.disabled(),
      },
      {
        key: LocationMfaType.INTERNAL,
        value: LocationMfaType.INTERNAL,
        label: LL.components.locationMfaTypeSelect.options.internal(),
      },
      {
        key: LocationMfaType.EXTERNAL,
        value: LocationMfaType.EXTERNAL,
        label: LL.components.locationMfaTypeSelect.options.external(),
      },
    ],
    [LL.components.aclDefaultPolicySelect.options],
  );
  return (
    <FormSelect
      controller={controller}
      options={options}
      label={LL.components.locationMfaTypeSelect.label()}
      disabled={disabled}
    />
  );
};
