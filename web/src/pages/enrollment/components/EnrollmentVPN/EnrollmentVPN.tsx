import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useCallback, useMemo, useState } from 'react';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { Select } from '../../../../shared/defguard-ui/components/Layout/Select/Select';
import {
  SelectSelectedValue,
  SelectSizeVariant,
} from '../../../../shared/defguard-ui/components/Layout/Select/types';
import useApi from '../../../../shared/hooks/useApi';
import { useToaster } from '../../../../shared/hooks/useToaster';
import { QueryKeys } from '../../../../shared/queries';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';

export const EnrollmentVPN = () => {
  const [isLoading, setLoading] = useState(false);

  const { LL } = useI18nContext();

  const componentLL = LL.enrollmentPage.settings.vpnOptionality;

  const {
    settings: { editSettings },
  } = useApi();

  const queryClient = useQueryClient();

  const toaster = useToaster();

  const settings = useEnrollmentStore((state) => state.settings);

  const { mutate } = useMutation({
    mutationFn: editSettings,
    onSuccess: () => {
      queryClient.invalidateQueries([QueryKeys.FETCH_SETTINGS]);
      toaster.success(LL.enrollmentPage.messages.edit.success());
      setLoading(false);
    },
    onError: () => {
      toaster.error(LL.enrollmentPage.messages.edit.error());
      setLoading(false);
    },
  });

  const vpnOptionalityOptions = useMemo(
    () => [
      {
        key: 1,
        value: true,
        label: componentLL.select.options.optional(),
      },
      {
        key: 2,
        value: false,
        label: componentLL.select.options.mandatory(),
      },
    ],
    [componentLL.select.options],
  );

  const renderSelectedVpn = useCallback(
    (selected: boolean): SelectSelectedValue => {
      const option = vpnOptionalityOptions.find((o) => o.value === selected);

      if (!option) throw Error("Selected value doesn't exist");

      return {
        key: option.key,
        displayValue: option.label,
      };
    },
    [vpnOptionalityOptions],
  );

  const handleChange = async (val: boolean) => {
    if (!isLoading && settings) {
      setLoading(true);
      try {
        mutate({ ...settings, enrollment_vpn_step_optional: val });
      } catch (e) {
        setLoading(false);
        toaster.error(LL.enrollmentPage.messages.edit.error());
      }
    }
  };

  return (
    <div id="enrollment-vpn-settings">
      <header>
        <h3>{componentLL.title()}</h3>
      </header>
      <Select
        sizeVariant={SelectSizeVariant.SMALL}
        selected={settings?.enrollment_vpn_step_optional}
        options={vpnOptionalityOptions}
        renderSelected={renderSelectedVpn}
        onChangeSingle={(res) => handleChange(res)}
        loading={isLoading || isUndefined(settings)}
      />
    </div>
  );
};
