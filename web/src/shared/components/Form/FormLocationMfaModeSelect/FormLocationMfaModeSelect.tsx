import { useMemo } from 'react';
import type { FieldValues, UseControllerProps } from 'react-hook-form';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormSelect } from '../../../defguard-ui/components/Form/FormSelect/FormSelect';
import type { SelectOption } from '../../../defguard-ui/components/Layout/Select/types';
import { LocationMfaMode } from '../../../types';

type Props<T extends FieldValues> = {
  controller: UseControllerProps<T>;
  disabled?: boolean;
};

export const FormLocationMfaModeSelect = <T extends FieldValues>({
  controller,
  disabled = false,
}: Props<T>) => {
  const { LL } = useI18nContext();

  const options = useMemo(
    (): SelectOption<LocationMfaMode>[] => [
      {
        key: LocationMfaMode.DISABLED,
        value: LocationMfaMode.DISABLED,
        label: LL.components.locationMfaModeSelect.options.disabled(),
      },
      {
        key: LocationMfaMode.INTERNAL,
        value: LocationMfaMode.INTERNAL,
        label: LL.components.locationMfaModeSelect.options.internal(),
      },
      {
        key: LocationMfaMode.EXTERNAL,
        value: LocationMfaMode.EXTERNAL,
        label: LL.components.locationMfaModeSelect.options.external(),
      },
    ],
    [LL.components.aclDefaultPolicySelect.options],
  );
  return (
    <FormSelect
      controller={controller}
      options={options}
      label={LL.components.locationMfaModeSelect.label()}
      disabled={disabled}
    />
  );
};
